use std::time::{Duration, Instant};

/// Un parámetro animable: el stream fija un OBJETIVO y un resorte críticamente
/// amortiguado lleva `value` hasta él. Así un dato suelto produce un movimiento suave.
#[derive(Debug, Clone, Copy)]
pub struct Param {
    /// Valor actual (lo que se dibuja).
    pub value: f32,
    target: f32,
    /// Valor de reposo: al vencer el TTL el objetivo vuelve aquí.
    base: f32,
    vel: f32,
    /// Rigidez del resorte (rad/s). Más alto = más rápido.
    omega: f32,
    expires: Option<Instant>,
}

impl Param {
    pub const fn new(base: f32) -> Self {
        Self::with_speed(base, 14.0)
    }

    pub const fn with_speed(base: f32, omega: f32) -> Self {
        Self { value: base, target: base, base, vel: 0.0, omega, expires: None }
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    pub fn set(&mut self, v: f32, ttl: Option<Duration>, now: Instant) {
        self.target = v;
        self.expires = ttl.map(|t| now + t);
    }

    pub fn tick(&mut self, dt: f32, now: Instant) {
        if self.expires.is_some_and(|t| now >= t) {
            self.target = self.base;
            self.expires = None;
        }
        // Resorte críticamente amortiguado, integración implícita (estable con cualquier dt).
        let (x, v, w) = (self.value, self.vel, self.omega);
        let f = 1.0 + 2.0 * dt * w;
        let dtw2 = dt * w * w;
        let det = f + dt * dtw2;
        self.value = (f * x + dt * v + dt * dtw2 * self.target) / det;
        self.vel = (v + dtw2 * (self.target - x)) / det;
        if self.settled() {
            self.value = self.target;
            self.vel = 0.0;
        }
    }

    pub fn settled(&self) -> bool {
        (self.value - self.target).abs() < 1e-3 && self.vel.abs() < 1e-3
    }

    /// Tiene un TTL pendiente (habrá que volver a la base aunque nada se mueva ahora).
    pub fn pending(&self) -> bool {
        self.expires.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reaches_target_and_returns_after_ttl() {
        let mut p = Param::new(0.0);
        let t0 = Instant::now();
        p.set(1.0, Some(Duration::from_millis(1500)), t0);
        for i in 1..=60 {
            p.tick(1.0 / 60.0, t0 + Duration::from_millis(i * 16));
        }
        assert!((p.value - 1.0).abs() < 0.01, "value={}", p.value);
        for i in 0..120 {
            p.tick(1.0 / 60.0, t0 + Duration::from_millis(1600 + i * 16));
        }
        assert!(p.value.abs() < 0.01, "value={}", p.value);
        assert!(p.settled());
    }
}
