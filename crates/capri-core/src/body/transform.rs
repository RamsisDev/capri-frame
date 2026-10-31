use super::Node;
use crate::Param;
use capri_protocol::Stream::{self, Body, Morph};

/// Transform global del cuerpo + núcleos activos.
#[derive(Debug, Clone)]
pub struct TransformController {
    pub shape_morph: ShapeMorph,
    /// Posición en la ventana, -1..1 (centro = 0).
    pub pos_x: Param,
    pub pos_y: Param,
    /// Rotación en el plano (grados).
    pub rot: Param,
    /// Giro sobre el eje vertical (grados): da la sensación de volumen.
    pub yaw: Param,
    /// Cabeceo de la cabeza (grados): positivo = mira arriba.
    pub pitch: Param,
    /// Inclinación lateral de la cabeza sobre el eje de la cara (grados): positivo = antihorario.
    pub roll: Param,
    pub scale: Param,
    pub opacity: Param,
}

/// Peso de cada núcleo (0..1). Los pesos se mezclan: cambiar de núcleo es un morph, no un corte.
#[derive(Debug, Clone)]
pub struct ShapeMorph {
    pub full: Param,
    pub eyes: Param,
    pub bolt: Param,
}

impl Default for TransformController {
    fn default() -> Self {
        Self {
            shape_morph: ShapeMorph {
                full: Param::with_speed(1.0, 6.0),
                eyes: Param::with_speed(0.0, 6.0),
                bolt: Param::with_speed(0.0, 6.0),
            },
            pos_x: Param::with_speed(0.0, 8.0),
            pos_y: Param::with_speed(0.0, 8.0),
            rot: Param::new(0.0),
            yaw: Param::with_speed(0.0, 8.0),
            pitch: Param::with_speed(0.0, 8.0),
            roll: Param::with_speed(0.0, 8.0),
            scale: Param::new(1.0),
            opacity: Param::new(1.0),
        }
    }
}

impl Node for TransformController {
    fn visit(&mut self, f: &mut dyn FnMut(Stream, &'static str, &mut Param)) {
        f(Morph, "core.full", &mut self.shape_morph.full);
        f(Morph, "core.eyes", &mut self.shape_morph.eyes);
        f(Morph, "core.bolt", &mut self.shape_morph.bolt);
        f(Morph, "opacity", &mut self.opacity);
        f(Body, "pos.x", &mut self.pos_x);
        f(Body, "pos.y", &mut self.pos_y);
        f(Body, "rot", &mut self.rot);
        f(Body, "yaw", &mut self.yaw);
        f(Body, "pitch", &mut self.pitch);
        f(Body, "roll", &mut self.roll);
        f(Body, "scale", &mut self.scale);
    }
}
