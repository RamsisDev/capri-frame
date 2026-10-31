//! Presets gráficos: el LOOK separado de la FORMA. Se recargan en caliente.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Style {
    pub palette: Palette,
    /// Grosor del contorno en píxeles.
    pub outline: f32,
    /// Halo alrededor del cuerpo, 0..1.
    pub glow: f32,
    pub scanlines: Scanlines,
    pub glitch: Glitch,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Palette {
    pub body: String,
    pub rim: String,
    /// Placa de la cara.
    pub mask: String,
    /// Iris (disco central de color). También tiñe el borde del rayo.
    pub eyes: String,
    /// Base del pelo.
    pub hair: String,
    /// Manchas estrelladas del pelo.
    pub hair_glint: String,
    /// Superficie oscura de la cara, alrededor de la placa.
    pub face: String,
    /// Silueta alrededor de cada ojo y disco bajo el iris. Sin valor = `body`.
    pub eye_shell: Option<String>,
    /// Anillo fino que rodea el disco del ojo. Sin valor = `mask`.
    pub eye_ring: Option<String>,
    /// Disco central del ojo, dentro del iris.
    pub pupil: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Scanlines {
    /// Líneas por alto de ventana.
    pub density: f32,
    pub strength: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Glitch {
    /// Probabilidad por banda y tick de que una franja se desplace, 0..1.
    pub rate: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            palette: Palette::default(),
            outline: 1.5,
            glow: 0.6,
            scanlines: Scanlines::default(),
            glitch: Glitch::default(),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            body: "#07070b".into(),
            rim: "#ff3df0".into(),
            mask: "#11e965".into(),
            eyes: "#7fe8ff".into(),
            hair: "#ef003d".into(),
            hair_glint: "#edc4ea".into(),
            face: "#070211".into(),
            eye_shell: None,
            eye_ring: None,
            pupil: "#9a9a9a".into(),
        }
    }
}

impl Default for Scanlines {
    fn default() -> Self {
        Self { density: 220.0, strength: 0.1 }
    }
}

impl Default for Glitch {
    fn default() -> Self {
        Self { rate: 0.03 }
    }
}

/// `"#rrggbb"` → `[r, g, b, 1]` en 0..1. Color inválido = magenta (se nota a la vista).
pub fn hex(s: &str) -> [f32; 4] {
    let s = s.trim_start_matches('#');
    let c = |i: usize| s.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
    match (s.len(), c(0), c(2), c(4)) {
        (6, Some(r), Some(g), Some(b)) => [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0],
        _ => [1.0, 0.0, 1.0, 1.0],
    }
}

/// Un preset en disco que se recarga cuando cambia el archivo.
pub struct StyleFile {
    path: PathBuf,
    modified: Option<SystemTime>,
    pub style: Style,
}

impl StyleFile {
    pub fn open(path: impl AsRef<Path>) -> Self {
        let mut f = Self { path: path.as_ref().to_owned(), modified: None, style: Style::default() };
        if !f.reload_if_changed() {
            eprintln!("[style] no se pudo leer {}; usando el estilo por defecto", f.path.display());
        }
        f
    }

    /// Devuelve true si recargó.
    pub fn reload_if_changed(&mut self) -> bool {
        let Ok(modified) = std::fs::metadata(&self.path).and_then(|m| m.modified()) else {
            return false;
        };
        if self.modified == Some(modified) {
            return false;
        }
        self.modified = Some(modified);
        match std::fs::read_to_string(&self.path).map_err(|e| e.to_string()).and_then(|t| {
            serde_json::from_str::<Style>(&t).map_err(|e| e.to_string())
        }) {
            Ok(style) => {
                self.style = style;
                eprintln!("[style] cargado {}", self.path.display());
                true
            }
            Err(e) => {
                eprintln!("[style] {} inválido: {e}", self.path.display());
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hex_parses() {
        assert_eq!(super::hex("#ff0000"), [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(super::hex("nope"), [1.0, 0.0, 1.0, 1.0]);
    }

    /// Los presets de `assets/styles` cargan, incluidos los que no traen las claves nuevas.
    #[test]
    fn bundled_styles_parse() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/styles");
        for name in ["construct.json", "mono.json", "capri-eyes.json"] {
            let text = std::fs::read_to_string(format!("{dir}/{name}")).unwrap();
            let style: super::Style = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_ne!(super::hex(&style.palette.pupil), [1.0, 0.0, 1.0, 1.0], "{name}");
        }
    }
}
