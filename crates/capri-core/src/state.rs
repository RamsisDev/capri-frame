use std::collections::HashSet;
use std::time::{Duration, Instant};

use capri_protocol::{Envelope, Scalar, Stream};

use crate::body::{EntityBody, Node};
use crate::idle::Idle;
use crate::presets;

/// Estado vivo del cuerpo: el árbol + idle + control de secuencia por stream.
#[derive(Debug, Default)]
pub struct State {
    pub body: EntityBody,
    pub idle: Idle,
    last_seq: [u64; 4],
    warned: HashSet<String>,
}

/// Claves que son presets (se aplican antes que los valores crudos del mismo mensaje,
/// así `{"core":"eyes","silhouette":0.3}` = solo ojos + silueta tenue).
const PRESET_KEYS: [(Stream, &str); 4] =
    [(Stream::Face, "expr"), (Stream::Body, "pose"), (Stream::Morph, "core"), (Stream::Voice, "viseme")];

impl State {
    pub fn apply(&mut self, env: &Envelope, now: Instant) {
        let slot = &mut self.last_seq[env.s as usize];
        if env.seq != 0 {
            if env.seq <= *slot {
                return; // mensaje viejo o repetido
            }
            *slot = env.seq;
        }
        let ttl = env.ttl.map(|ms| Duration::from_millis(ms as u64));

        let (presets, raw): (Vec<_>, Vec<_>) = env
            .entries()
            .filter_map(|e| e.map_err(|k| self.warn(env.s, &k, "valor no soportado")).ok())
            .partition(|(k, _)| PRESET_KEYS.contains(&(env.s, k.as_str())));

        for (key, value) in presets.into_iter().chain(raw) {
            self.set(env.s, &key, value, ttl, now);
        }
    }

    fn set(&mut self, stream: Stream, key: &str, value: Scalar, ttl: Option<Duration>, now: Instant) {
        let v = match (stream, key, value) {
            (s, k, Scalar::Text(name)) => {
                let preset = match (s, k) {
                    (Stream::Face, "expr") => presets::expr(&name),
                    (Stream::Body, "pose") => presets::pose(&name),
                    (Stream::Morph, "core") => presets::core(&name),
                    (Stream::Voice, "viseme") => presets::viseme(&name),
                    _ => return self.warn(s, k, "no acepta texto"),
                };
                match preset {
                    Some(list) => {
                        for &(k, v) in list {
                            self.set(s, k, Scalar::Num(v), ttl, now);
                        }
                    }
                    None => self.warn(s, &format!("{k}={name}"), "preset desconocido"),
                }
                return;
            }
            (_, _, Scalar::Num(v)) => v,
            (_, _, Scalar::Bool(b)) => b as u8 as f32,
        };
        if !self.body.with_param(stream, key, |p| p.set(v, ttl, now)) {
            self.warn(stream, key, "clave desconocida");
        }
    }

    /// Avanza resortes, TTLs e idle. Devuelve true si algo sigue en movimiento.
    pub fn tick(&mut self, dt: f32, now: Instant) -> bool {
        self.idle.tick(dt);
        let mut moving = false;
        self.body.visit(&mut |_, _, p| {
            p.tick(dt, now);
            moving |= !p.settled() || p.pending();
        });
        moving
    }

    fn warn(&mut self, stream: Stream, key: &str, why: &str) {
        let id = format!("{stream:?}/{key}");
        if self.warned.insert(id.clone()) {
            eprintln!("[core] {id}: {why}. Claves válidas: {:?}", EntityBody::keys(stream));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use capri_protocol::decode;

    fn send(state: &mut State, json: &str, now: Instant) {
        for env in decode(json).unwrap() {
            state.apply(&env, now);
        }
    }

    #[test]
    fn preset_then_raw_override() {
        let mut s = State::default();
        let now = Instant::now();
        send(&mut s, r#"{"s":"morph","set":{"silhouette":0.3,"core":"eyes"}}"#, now);
        assert_eq!(s.body.transform.shape_morph.eyes.target(), 1.0);
        assert_eq!(s.body.silhouette.opacity.target(), 0.3, "valor crudo gana al preset");
    }

    #[test]
    fn streams_cannot_touch_foreign_keys() {
        let mut s = State::default();
        send(&mut s, r#"{"s":"face","set":{"arm.l":90}}"#, Instant::now());
        assert_eq!(s.body.silhouette.arm_left.target(), 0.0);
    }

    #[test]
    fn viseme_preset_sets_one_weight() {
        let mut s = State::default();
        let now = Instant::now();
        send(&mut s, r#"{"s":"voice","ttl":150,"set":{"viseme":"F/L"}}"#, now);
        let w: Vec<f32> = s.body.face.mouth.viseme.iter().map(|p| p.target()).collect();
        assert_eq!(w, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
        send(&mut s, r#"{"s":"voice","set":{"viseme":"A"}}"#, now);
        assert_eq!(s.body.face.mouth.viseme[2].target(), 1.0);
        assert_eq!(s.body.face.mouth.viseme[7].target(), 0.0);
    }

    #[test]
    fn old_seq_is_dropped() {
        let mut s = State::default();
        let now = Instant::now();
        send(&mut s, r#"{"s":"face","seq":5,"set":{"mouth.open":1}}"#, now);
        send(&mut s, r#"{"s":"face","seq":4,"set":{"mouth.open":0.2}}"#, now);
        assert_eq!(s.body.face.mouth.open.target(), 1.0);
    }
}
