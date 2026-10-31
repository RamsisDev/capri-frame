//! Servidor WebSocket. No sabe nada del cuerpo: valida mensajes y los empuja a un canal.

use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

use capri_protocol::{decode, Envelope};
use tungstenite::Message;

/// Arranca el servidor en un hilo propio.
/// `wake` se llama tras encolar mensajes para despertar el loop de render.
pub fn spawn(
    addr: &str,
    tx: Sender<Envelope>,
    wake: impl Fn() + Send + Sync + 'static,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    eprintln!("[link] escuchando en ws://{addr}");
    let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (tx, wake) = (tx.clone(), wake.clone());
            thread::spawn(move || {
                if let Err(e) = serve(stream, &tx, &*wake) {
                    eprintln!("[link] conexión cerrada: {e}");
                }
            });
        }
    });
    Ok(())
}

fn serve(stream: TcpStream, tx: &Sender<Envelope>, wake: &dyn Fn()) -> tungstenite::Result<()> {
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    stream.set_nodelay(true).ok();
    let mut ws = tungstenite::accept(stream).map_err(|e| match e {
        tungstenite::HandshakeError::Failure(e) => e,
        tungstenite::HandshakeError::Interrupted(_) => tungstenite::Error::ConnectionClosed,
    })?;
    eprintln!("[link] entidad conectada: {peer}");
    loop {
        let text = match ws.read()? {
            Message::Text(t) => t.to_string(),
            Message::Binary(b) => String::from_utf8_lossy(&b).into_owned(),
            Message::Close(_) => return Ok(()),
            _ => continue,
        };
        match decode(&text) {
            Ok(envs) => {
                for env in envs {
                    if tx.send(env).is_err() {
                        return Ok(()); // el cuerpo se cerró
                    }
                }
                wake();
            }
            Err(e) => eprintln!("[link] mensaje inválido ({e}): {text}"),
        }
    }
}
