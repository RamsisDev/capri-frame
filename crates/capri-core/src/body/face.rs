use super::Node;
use crate::Param;
use capri_protocol::Stream::{self, Face as F, Voice};

/// La cara: los dos ojos con su placa (Mask) y la boca. Sobrevive en todos los núcleos.
/// La placa es parte de los ojos: siempre se dibuja con ellos. La boca va bajo la placa.
#[derive(Debug, Clone)]
pub struct Face {
    pub mask: Mask,
    pub eye_left: Eye,
    pub eye_right: Eye,
    pub mouth: Mouth,
    /// Curva del ojo al cerrarse: -1 = triste (U), 0 = raya recta, 1 = feliz (∩).
    pub lid: Param,
    /// Dirección de la mirada, -1..1.
    pub gaze_x: Param,
    pub gaze_y: Param,
}

#[derive(Debug, Clone)]
pub struct Mask {
    pub glow: Param,
}

#[derive(Debug, Clone)]
pub struct Eye {
    /// 0 = cerrado, 1 = abierto.
    pub open: Param,
    /// Tamaño relativo (1 = normal).
    pub size: Param,
}

#[derive(Debug, Clone)]
pub struct Mouth {
    /// 0 = cerrada, 1 = abierta.
    pub open: Param,
    /// -1 = triste, 0 = recta, 1 = sonrisa.
    pub curve: Param,
    /// Nivel de voz 0..1 (stream `voice`). Se SUMA a `open`.
    pub voice: Param,
    /// Pesos de cada visema (stream `voice`), en el orden de [`VISEMES`]. Suman 0..1: lo que
    /// falta hasta 1 es la boca de reposo (X, que se abre con `open`).
    pub viseme: [Param; VISEMES.len()],
}

/// Visemas, en el orden de la referencia: X (reposo), MBP, A, E, I, O, U, F/L.
pub const VISEMES: [&str; 8] = ["x", "mbp", "a", "e", "i", "o", "u", "fl"];
const VISEME_KEYS: [&str; 8] =
    ["viseme.x", "viseme.mbp", "viseme.a", "viseme.e", "viseme.i", "viseme.o", "viseme.u", "viseme.fl"];

impl Default for Face {
    fn default() -> Self {
        let eye = || Eye { open: Param::with_speed(1.0, 22.0), size: Param::new(1.0) };
        Self {
            mask: Mask { glow: Param::new(0.6) },
            eye_left: eye(),
            eye_right: eye(),
            lid: Param::new(0.0),
            mouth: Mouth {
                open: Param::new(0.0),
                curve: Param::new(0.0),
                voice: Param::with_speed(0.0, 30.0),
                viseme: [Param::with_speed(0.0, 40.0); VISEMES.len()],
            },
            gaze_x: Param::with_speed(0.0, 18.0),
            gaze_y: Param::with_speed(0.0, 18.0),
        }
    }
}

impl Node for Face {
    fn visit(&mut self, f: &mut dyn FnMut(Stream, &'static str, &mut Param)) {
        f(F, "mask.glow", &mut self.mask.glow);
        f(F, "eye.l.open", &mut self.eye_left.open);
        f(F, "eye.r.open", &mut self.eye_right.open);
        f(F, "eye.l.size", &mut self.eye_left.size);
        f(F, "eye.r.size", &mut self.eye_right.size);
        f(F, "eye.lid", &mut self.lid);
        f(F, "gaze.x", &mut self.gaze_x);
        f(F, "gaze.y", &mut self.gaze_y);
        f(F, "mouth.open", &mut self.mouth.open);
        f(F, "mouth.curve", &mut self.mouth.curve);
        f(Voice, "level", &mut self.mouth.voice);
        for (key, p) in VISEME_KEYS.into_iter().zip(&mut self.mouth.viseme) {
            f(Voice, key, p);
        }
    }
}
