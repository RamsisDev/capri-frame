//! capri-ctl: controlador de prueba. Habla con el cuerpo igual que lo haría la entidad.
//!
//! ```text
//! capri-ctl send '<json>'          un mensaje suelto
//! capri-ctl play <escenario.jsonl> reproduce un escenario (--loop para repetir)
//! capri-ctl all                    todos los escenarios de controllers/scenarios, en orden
//! capri-ctl pipe                   stdin (una línea JSON por mensaje) → cuerpo
//!   --url ws://127.0.0.1:7777
//! ```
//!
//! Formato de escenario (.jsonl):
//! - `# texto`      → se imprime como "VERIFICAR: texto" (qué mirar en pantalla)
//! - `{"wait": ms}` → pausa
//! - cualquier otro objeto → se manda tal cual; si trae `"wait"`, espera antes de mandarlo

use std::io::BufRead;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use serde_json::Value;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

type Ws = WebSocket<MaybeTlsStream<TcpStream>>;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let url = take_flag(&mut args, "--url")
        .unwrap_or_else(|| format!("ws://127.0.0.1:{}", capri_protocol::DEFAULT_PORT));
    let repeat = take_switch(&mut args, "--loop");

    let result = match args.first().map(String::as_str) {
        Some("send") => args.get(1).ok_or("falta el JSON".to_string()).and_then(|j| {
            let mut ws = connect(&url)?;
            send_line(&mut ws, j)?;
            close(ws)
        }),
        Some("play") => args.get(1).ok_or("falta el archivo".to_string()).and_then(|f| {
            let mut ws = connect(&url)?;
            loop {
                play(&mut ws, Path::new(f))?;
                if !repeat {
                    break close(ws);
                }
            }
        }),
        Some("all") => connect(&url).and_then(|mut ws| {
            for f in scenarios()? {
                play(&mut ws, &f)?;
            }
            close(ws)
        }),
        Some("pipe") => connect(&url).and_then(|mut ws| {
            for line in std::io::stdin().lock().lines() {
                let line = line.map_err(|e| e.to_string())?;
                if !line.trim().is_empty() {
                    send_line(&mut ws, &line)?;
                }
            }
            close(ws)
        }),
        _ => Err("uso: capri-ctl send '<json>' | play <archivo.jsonl> [--loop] | all | pipe  [--url ws://...]".into()),
    };
    if let Err(e) = result {
        eprintln!("capri-ctl: {e}");
        std::process::exit(1);
    }
}

/// Cierre limpio: espera a que el cuerpo confirme el cierre.
fn close(mut ws: Ws) -> Result<(), String> {
    ws.close(None).map_err(|e| e.to_string())?;
    while ws.read().is_ok() {}
    Ok(())
}

fn connect(url: &str) -> Result<Ws, String> {
    tungstenite::connect(url)
        .map(|(ws, _)| ws)
        .map_err(|e| format!("no se pudo conectar a {url} (¿está corriendo `capri`?): {e}"))
}

/// Valida contra el protocolo antes de mandar: si el controlador manda basura, falla aquí.
fn send_line(ws: &mut Ws, json: &str) -> Result<(), String> {
    capri_protocol::decode(json).map_err(|e| format!("JSON inválido ({e}): {json}"))?;
    ws.send(Message::text(json)).map_err(|e| e.to_string())
}

fn play(ws: &mut Ws, file: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    println!("\n▶ {}", file.display());
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(note) = line.strip_prefix('#') {
            println!("  VERIFICAR:{note}");
            continue;
        }
        let mut v: Value = serde_json::from_str(line).map_err(|e| format!("{}:{}: {e}", file.display(), n + 1))?;
        if let Some(ms) = v.as_object_mut().and_then(|o| o.remove("wait")).and_then(|w| w.as_u64()) {
            sleep(Duration::from_millis(ms));
        }
        if v.as_object().is_some_and(|o| o.is_empty()) {
            continue; // era solo una pausa
        }
        send_line(ws, &v.to_string())?;
    }
    Ok(())
}

fn scenarios() -> Result<Vec<PathBuf>, String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .collect();
    files.sort();
    Ok(files)
}

fn take_flag(args: &mut Vec<String>, name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    args.remove(i);
    (i < args.len()).then(|| args.remove(i))
}

fn take_switch(args: &mut Vec<String>, name: &str) -> bool {
    let before = args.len();
    args.retain(|a| a != name);
    args.len() != before
}
