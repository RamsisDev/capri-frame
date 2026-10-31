//! Solo se administra el proceso que lanzó esta sesión; nunca un Capri ajeno.

use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command};

pub struct Preview(Child);

impl Preview {
    pub fn spawn(port: Option<u16>, style: &Path) -> Result<(Self, u16), String> {
        // Evita tomar el puerto de otra entidad. El sistema elige uno libre por defecto.
        let reservation = TcpListener::bind(("127.0.0.1", port.unwrap_or(0)))
            .map_err(|e| format!("no se pudo reservar el puerto de prueba: {e}"))?;
        let port = reservation.local_addr().map_err(|e| e.to_string())?.port();
        let executable = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .with_file_name(format!("capri{}", std::env::consts::EXE_SUFFIX));
        if !executable.is_file() {
            return Err("falta el binario capri junto al panel; ejecuta `make test`".into());
        }
        if !style.is_file() {
            return Err(format!("no existe el estilo {}", style.display()));
        }
        drop(reservation);
        let mut command = Command::new(executable);
        command
            .args([
                "--port",
                &port.to_string(),
                "--size",
                "640x640",
                "--interactive",
                "--style",
            ])
            .arg(style);
        // Reutiliza la terminal de make; no abre otra consola en Windows.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let child = command
            .spawn()
            .map_err(|e| format!("no se pudo abrir Capri: {e}"))?;
        Ok((Self(child), port))
    }

    pub fn exited(&mut self) -> bool {
        self.0.try_wait().ok().flatten().is_some()
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
