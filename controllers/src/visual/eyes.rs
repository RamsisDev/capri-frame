//! Objetivos del controlador. El cuerpo sigue siendo la única fuente de estado animado.

use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq)]
pub struct Eyes {
    pub open: [f32; 2],
    pub size: [f32; 2],
    pub gaze: [f32; 2],
    pub core: &'static str,
    pub scale: f32,
}

impl Default for Eyes {
    fn default() -> Self {
        Self {
            open: [1.0; 2],
            size: [1.0; 2],
            gaze: [0.0; 2],
            core: "eyes",
            scale: 2.0,
        }
    }
}

impl Eyes {
    pub fn messages(&self) -> Value {
        // Los controles tienen pasos de centésima: el JSON debe mostrar 0.4, no ruido de f32.
        let decimal = |value: f32| (value as f64 * 100.0).round() / 100.0;
        json!([
            {"s": "morph", "set": {"core": self.core}},
            {"s": "body", "set": {"scale": decimal(self.scale)}},
            {"s": "face", "set": {
                "eye.l.open": decimal(self.open[0]), "eye.r.open": decimal(self.open[1]),
                "eye.l.size": decimal(self.size[0]), "eye.r.size": decimal(self.size[1]),
                "gaze.x": decimal(self.gaze[0]), "gaze.y": decimal(self.gaze[1])
            }}
        ])
    }

    pub fn json(&self) -> String {
        self.messages().to_string()
    }

    pub fn scenario(&self) -> String {
        format!(
            "# Ajuste de ojos aprobado desde capri-eyes. Verificar apertura, tamaño y mirada.\n{}\n",
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
        assert_eq!(face.gaze_x.target(), -1.0);
        assert_eq!(face.gaze_y.target(), 0.7);
        assert_eq!(state.body.transform.shape_morph.eyes.target(), 1.0);
        assert_eq!(state.body.transform.scale.target(), 2.0);
    }

    #[test]
    fn saved_scenario_replays_with_the_existing_cli_format() {
        let eyes = Eyes::default();
        let scenario = eyes.scenario();
        let lines: Vec<_> = scenario.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(decode(lines[0]).unwrap().len(), 3);
    }
}
