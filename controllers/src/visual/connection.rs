//! Red fuera del hilo gráfico; conserva solo el último objetivo y lo reenvía al reconectar.

use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tungstenite::{Message, WebSocket};

pub enum Event {
    Connected,
    Sent(String),
    Disconnected(String),
}

enum Command {
    Set(String),
    Stop,
}

pub struct Connection {
    tx: Sender<Command>,
    pub events: Receiver<Event>,
    worker: Option<JoinHandle<()>>,
}

impl Connection {
    pub fn start(port: u16, initial: String) -> Self {
        let (tx, rx) = mpsc::channel();
        let (events_tx, events) = mpsc::channel();
        let worker = thread::spawn(move || run(port, initial, rx, events_tx));
        Self {
            tx,
            events,
            worker: Some(worker),
        }
    }

    pub fn set(&self, json: String) {
        let _ = self.tx.send(Command::Set(json));
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn connect(port: u16) -> Result<WebSocket<TcpStream>, String> {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let stream =
        TcpStream::connect_timeout(&addr, Duration::from_millis(300)).map_err(|e| e.to_string())?;
    stream.set_nodelay(true).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_millis(300)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_millis(300)))
        .map_err(|e| e.to_string())?;
    let (ws, _) = tungstenite::client(format!("ws://{addr}"), stream).map_err(|e| e.to_string())?;
    ws.get_ref()
        .set_read_timeout(Some(Duration::from_millis(30)))
        .map_err(|e| e.to_string())?;
    Ok(ws)
}

fn run(port: u16, mut latest: String, rx: Receiver<Command>, events: Sender<Event>) {
    let mut ws: Option<WebSocket<TcpStream>> = None;
    let mut retry = Instant::now();
    let mut dirty = true;
    loop {
        // El timeout limita a ~30 Hz los envíos al arrastrar sliders.
        match rx.recv_timeout(Duration::from_millis(30)) {
            Ok(Command::Set(json)) => {
                latest = json;
                dirty = true;
            }
            Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        for command in rx.try_iter() {
            match command {
                Command::Set(json) => {
                    latest = json;
                    dirty = true;
                }
                Command::Stop => return,
            }
        }
        if ws.is_none() && Instant::now() >= retry {
            match connect(port) {
                Ok(socket) => {
                    ws = Some(socket);
                    dirty = true;
                    let _ = events.send(Event::Connected);
                }
                Err(error) => {
                    let _ = events.send(Event::Disconnected(error));
                    retry = Instant::now() + Duration::from_secs(1);
                }
            }
        }
        let Some(socket) = &mut ws else { continue };
        let result = (|| -> Result<(), String> {
            if dirty {
                capri_protocol::decode(&latest).map_err(|e| e.to_string())?;
                socket
                    .send(Message::text(latest.clone()))
                    .map_err(|e| e.to_string())?;
                dirty = false;
                let _ = events.send(Event::Sent(latest.clone()));
            }
            match socket.read() {
                Ok(Message::Close(_)) => Err("Capri cerró la conexión".into()),
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    Ok(())
                }
                Err(e) => Err(e.to_string()),
                _ => socket.flush().map_err(|e| e.to_string()),
            }
        })();
        if let Err(error) = result {
            let _ = events.send(Event::Disconnected(error));
            ws = None;
            retry = Instant::now() + Duration::from_secs(1);
        }
    }
    if let Some(mut socket) = ws {
        let _ = socket.close(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn reconnect_sends_latest_settings_without_a_graphical_loop() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let initial = r#"{"s":"face","set":{"eye.l.open":1}}"#.to_string();
        let updated = r#"{"s":"face","set":{"eye.l.open":0}}"#.to_string();
        let expected = updated.clone();
        listener.set_nonblocking(true).unwrap();
        let server = thread::spawn(move || {
            let accept = || {
                let deadline = Instant::now() + Duration::from_secs(5);
                loop {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            // Windows hereda el modo no bloqueante del listener.
                            stream.set_nonblocking(false).unwrap();
                            stream
                                .set_read_timeout(Some(Duration::from_secs(3)))
                                .unwrap();
                            return tungstenite::accept(stream).unwrap();
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < deadline, "no se reconectó");
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(e) => panic!("{e}"),
                    }
                }
            };
            let mut first = accept();
            assert!(first.read().unwrap().is_text());
            first.close(None).unwrap();
            drop(first);
            let mut second = accept();
            assert_eq!(second.read().unwrap().into_text().unwrap(), expected);
        });
        let connection = Connection::start(port, initial);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(Instant::now() < deadline);
            if matches!(
                connection
                    .events
                    .recv_timeout(Duration::from_secs(3))
                    .unwrap(),
                Event::Disconnected(_)
            ) {
                connection.set(updated);
                break;
            }
        }
        server.join().unwrap();
    }
}
