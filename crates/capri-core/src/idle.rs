//! Comportamiento local cuando la entidad no manda nada: el cuerpo no se congela.
//! Se SUMA a lo que manden los streams, nunca lo pisa.

#[derive(Debug, Clone)]
pub struct Idle {
    t: f32,
    next_blink: f32,
    blink_start: f32,
    rng: u32,
    /// 0 = ojos abiertos, 1 = cerrados por el parpadeo.
    pub blink: f32,
    /// 0..1 ciclo de respiración.
    pub breath: f32,
}

impl Default for Idle {
    fn default() -> Self {
        Self { t: 0.0, next_blink: 2.0, blink_start: -1.0, rng: 0x9E37_79B9, blink: 0.0, breath: 0.5 }
    }
}

const BLINK_SECS: f32 = 0.16;

impl Idle {
    pub fn tick(&mut self, dt: f32) {
        self.t += dt;
        self.breath = (self.t * 1.4).sin() * 0.5 + 0.5;

        if self.t >= self.next_blink {
            self.blink_start = self.t;
            self.next_blink = self.t + 2.5 + self.rand() * 4.0;
        }
        let k = (self.t - self.blink_start) / BLINK_SECS;
        self.blink = if (0.0..1.0).contains(&k) { 1.0 - (k * 2.0 - 1.0).abs() } else { 0.0 };
    }

    /// Segundos de vida (para animaciones procedurales del render).
    pub fn time(&self) -> f32 {
        self.t
    }

    fn rand(&mut self) -> f32 {
        // xorshift32: suficiente para variar parpadeos, sin dependencias.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng as f32) / (u32::MAX as f32)
    }
}
