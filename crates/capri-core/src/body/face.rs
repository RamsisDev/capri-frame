use super::Node;
use crate::Param;
use capri_protocol::Stream::{self, Face as F, Morph, Voice};

/// La cara: la placa (Mask) con dos ojos y boca. Sobrevive en todos los núcleos.
#[derive(Debug, Clone)]
pub struct Face {
    pub mask: Mask,
    pub eye_left: Eye,
    pub eye_right: Eye,
    pub mouth: Mouth,
    /// Dirección de la mirada, -1..1.
    pub gaze_x: Param,
    pub gaze_y: Param,
}

#[derive(Debug, Clone)]
pub struct Mask {
    /// 0 = sin placa (solo ojos). Lo controla `morph`: es parte del núcleo.
    pub visible: Param,
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
}

impl Default for Face {
    fn default() -> Self {
        let eye = || Eye { open: Param::with_speed(1.0, 22.0), size: Param::new(1.0) };
        Self {
            mask: Mask { visible: Param::with_speed(1.0, 8.0), glow: Param::new(0.6) },
            eye_left: eye(),
            eye_right: eye(),
            mouth: Mouth {
                open: Param::new(0.0),
                curve: Param::new(0.0),
                voice: Param::with_speed(0.0, 30.0),
            },
            gaze_x: Param::with_speed(0.0, 18.0),
            gaze_y: Param::with_speed(0.0, 18.0),
        }
    }
}

impl Node for Face {
    fn visit(&mut self, f: &mut dyn FnMut(Stream, &'static str, &mut Param)) {
        f(Morph, "mask", &mut self.mask.visible);
        f(F, "mask.glow", &mut self.mask.glow);
        f(F, "eye.l.open", &mut self.eye_left.open);
        f(F, "eye.r.open", &mut self.eye_right.open);
        f(F, "eye.l.size", &mut self.eye_left.size);
        f(F, "eye.r.size", &mut self.eye_right.size);
        f(F, "gaze.x", &mut self.gaze_x);
        f(F, "gaze.y", &mut self.gaze_y);
        f(F, "mouth.open", &mut self.mouth.open);
        f(F, "mouth.curve", &mut self.mouth.curve);
        f(Voice, "level", &mut self.mouth.voice);
    }
}
