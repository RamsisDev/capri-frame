use super::Node;
use crate::Param;
use capri_protocol::Stream::{self, Body, Morph};

/// El cuerpo: torso, brazos y piernas. Ángulos en grados.
#[derive(Debug, Clone)]
pub struct Silhouette {
    /// Opacidad de toda la silueta (la controla `morph`).
    pub opacity: Param,
    pub neck: Param,
    pub torso: Torso,
    pub arm_left: Param,
    pub arm_right: Param,
    pub leg_left: Param,
    pub leg_right: Param,
}

#[derive(Debug, Clone)]
pub struct Torso {
    pub lean: Param,
}

impl Default for Silhouette {
    fn default() -> Self {
        Self {
            opacity: Param::with_speed(1.0, 8.0),
            neck: Param::new(0.0),
            torso: Torso { lean: Param::with_speed(0.0, 10.0) },
            arm_left: Param::new(0.0),
            arm_right: Param::new(0.0),
            leg_left: Param::new(0.0),
            leg_right: Param::new(0.0),
        }
    }
}

impl Node for Silhouette {
    fn visit(&mut self, f: &mut dyn FnMut(Stream, &'static str, &mut Param)) {
        f(Morph, "silhouette", &mut self.opacity);
        f(Body, "neck", &mut self.neck);
        f(Body, "torso.lean", &mut self.torso.lean);
        f(Body, "arm.l", &mut self.arm_left);
        f(Body, "arm.r", &mut self.arm_right);
        f(Body, "leg.l", &mut self.leg_left);
        f(Body, "leg.r", &mut self.leg_right);
    }
}
