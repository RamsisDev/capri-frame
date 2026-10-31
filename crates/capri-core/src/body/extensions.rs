use super::Node;
use crate::Param;
use capri_protocol::Stream::{self, Morph};

/// Partes que la entidad hace crecer a gusto.
#[derive(Debug, Clone)]
pub struct Extensions {
    pub spikes: Spikes,
    pub fragments: Fragments,
}

/// Púas fusionadas al cuerpo (sobre la cabeza: el "peinado").
#[derive(Debug, Clone)]
pub struct Spikes {
    /// 0..1 qué tan presentes.
    pub amount: Param,
    pub length: Param,
}

/// Piezas sueltas que orbitan / se desprenden.
#[derive(Debug, Clone)]
pub struct Fragments {
    pub amount: Param,
    pub spread: Param,
}

impl Default for Extensions {
    fn default() -> Self {
        Self {
            spikes: Spikes { amount: Param::with_speed(1.0, 8.0), length: Param::with_speed(1.0, 8.0) },
            fragments: Fragments { amount: Param::with_speed(0.0, 6.0), spread: Param::with_speed(1.0, 6.0) },
        }
    }
}

impl Node for Extensions {
    fn visit(&mut self, f: &mut dyn FnMut(Stream, &'static str, &mut Param)) {
        f(Morph, "spikes", &mut self.spikes.amount);
        f(Morph, "spikes.len", &mut self.spikes.length);
        f(Morph, "fragments", &mut self.fragments.amount);
        f(Morph, "fragments.spread", &mut self.fragments.spread);
    }
}
