//! Objetivos del controlador. El cuerpo sigue siendo la única fuente de estado animado.

use serde_json::{Map, Value, json};

/// Visemas del panel: etiqueta del botón y clave cruda del stream `voice`, en el orden del cuerpo.
pub const VISEMES: [(&str, &str); 8] = [
    ("X", "viseme.x"),
    ("MBP", "viseme.mbp"),
    ("A", "viseme.a"),
    ("E", "viseme.e"),
    ("I", "viseme.i"),
    ("O", "viseme.o"),
    ("U", "viseme.u"),
    ("F/L", "viseme.fl"),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Eyes {
    pub open: [f32; 2],
    pub size: [f32; 2],
    /// Curva al cerrar: -1 triste, 0 neutro, 1 feliz.
    pub lid: f32,
    pub gaze: [f32; 2],
    pub core: &'static str,
    pub scale: f32,
    /// Giro de la cabeza en grados: 0 frontal, ±90 perfil, 180 espalda.
    pub yaw: f32,
    /// Cabeceo en grados: positivo = mira arriba.
    pub pitch: f32,
    /// Inclinación lateral en grados: positivo = antihorario.
    pub roll: f32,
    /// Visema elegido (índice en [`VISEMES`]); `None` = reposo, todos los pesos a 0.
    pub viseme: Option<usize>,
    /// Apertura de la boca de reposo: 0 cerrada (X) / 1 abierta (A).
    pub mouth_open: f32,
    /// Emoción de la boca: -1 triste, 0 neutra, 1 sonrisa.
    pub mouth_curve: f32,
}

impl Default for Eyes {
    fn default() -> Self {
        Self {
            open: [1.0; 2],
            size: [1.0; 2],
            lid: 0.0,
            gaze: [0.0; 2],
            core: "eyes",
            scale: 2.0,
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            viseme: None,
            mouth_open: 0.0,
            mouth_curve: 0.0,
        }
    }
}

impl Eyes {
    pub fn messages(&self) -> Value {
        // Los controles tienen pasos de centésima: el JSON debe mostrar 0.4, no ruido de f32.
        let decimal = |value: f32| (value as f64 * 100.0).round() / 100.0;
        // Pesos crudos (no el preset de texto): idempotente y comprobable clave a clave.
        let weights: Map<String, Value> = VISEMES
            .iter()
            .enumerate()
            .map(|(i, (_, key))| (key.to_string(), json!(if self.viseme == Some(i) { 1.0 } else { 0.0 })))
            .collect();
        json!([
            {"s": "morph", "set": {"core": self.core}},
            {"s": "body", "set": {
                "scale": decimal(self.scale), "yaw": decimal(self.yaw),
                "pitch": decimal(self.pitch), "roll": decimal(self.roll)
            }},
            {"s": "face", "set": {
                "eye.l.open": decimal(self.open[0]), "eye.r.open": decimal(self.open[1]),
                "eye.l.size": decimal(self.size[0]), "eye.r.size": decimal(self.size[1]),
                "eye.lid": decimal(self.lid),
                "gaze.x": decimal(self.gaze[0]), "gaze.y": decimal(self.gaze[1]),
                "mouth.open": decimal(self.mouth_open), "mouth.curve": decimal(self.mouth_curve)
            }},
            {"s": "voice", "set": weights}
        ])
    }

    pub fn json(&self) -> String {
        self.messages().to_string()
    }

    pub fn scenario(&self) -> String {
        format!(
            "# Ajuste de ojos aprobado desde capri-eyes. Verificar ojos, mirada y boca.\n{}\n",
            self.json()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use capri_core::{State, body::EntityBody};
    use capri_protocol::{Scalar, decode};
    use std::time::Instant;

    #[test]
    fn controls_reach_the_body_through_their_own_streams() {
        let eyes = Eyes {
            open: [0.0, 0.4],
            size: [0.5, 1.5],
            gaze: [-1.0, 0.7],
            lid: -1.0,
            yaw: -135.0,
            pitch: 30.0,
            roll: -15.0,
            viseme: Some(5),
            mouth_open: 0.6,
            mouth_curve: -1.0,
            ..Eyes::default()
        };
        let mut state = State::default();
        for env in decode(&eyes.json()).unwrap() {
            for entry in env.entries() {
                let (key, value) = entry.unwrap();
                if !matches!(value, Scalar::Text(_)) {
                    assert!(EntityBody::keys(env.s).contains(&key.as_str()), "{key}");
                }
            }
            state.apply(&env, Instant::now());
        }
        let face = &state.body.face;
        assert_eq!(face.eye_left.open.target(), 0.0);
        assert_eq!(face.eye_right.open.target(), 0.4);
        assert_eq!(face.eye_left.size.target(), 0.5);
        assert_eq!(face.eye_right.size.target(), 1.5);
        assert_eq!(face.lid.target(), -1.0);
        assert_eq!(face.gaze_x.target(), -1.0);
        assert_eq!(face.gaze_y.target(), 0.7);
        assert_eq!(face.mouth.open.target(), 0.6);
        assert_eq!(face.mouth.curve.target(), -1.0);
        for (i, weight) in face.mouth.viseme.iter().enumerate() {
            assert_eq!(weight.target(), if i == 5 { 1.0 } else { 0.0 }, "visema {i}");
        }
        assert_eq!(state.body.transform.shape_morph.eyes.target(), 1.0);
        assert_eq!(state.body.transform.scale.target(), 2.0);
        assert_eq!(state.body.transform.yaw.target(), -135.0);
        assert_eq!(state.body.transform.pitch.target(), 30.0);
        assert_eq!(state.body.transform.roll.target(), -15.0);
    }

    #[test]
    fn saved_scenario_replays_with_the_existing_cli_format() {
        let eyes = Eyes::default();
        let scenario = eyes.scenario();
        let lines: Vec<_> = scenario.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(decode(lines[0]).unwrap().len(), 4);
    }
}
