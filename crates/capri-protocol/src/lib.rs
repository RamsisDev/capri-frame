//! Protocolo de los streams que controlan el cuerpo.
//!
//! Un mensaje = un sobre (`Envelope`) con actualizaciones PARCIALES:
//! `{ "s": "face", "seq": 12, "ttl": 800, "set": { "eye.l.open": 0.2, "gaze": [0.3, -0.1] } }`

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const DEFAULT_PORT: u16 = 7777;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    /// Control facial: Face/{Mask, EyeLeft, EyeRight, Mouth}.
    Face,
    /// Control corporal: Silhouette/* + posición y rotación global.
    Body,
    /// Control polimórfico: núcleos, forma, opacidad, Extensions/*.
    Morph,
    /// Voz: capa aditiva sobre la boca y la máscara (nivel de audio, visemas).
    Voice,
}

impl Stream {
    pub const ALL: [Stream; 4] = [Stream::Face, Stream::Body, Stream::Morph, Stream::Voice];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Stream destino.
    pub s: Stream,
    /// Secuencia por stream. Mensajes con `seq` <= al último visto se descartan. 0 = sin control.
    #[serde(default)]
    pub seq: u64,
    /// Milisegundos de validez de los valores. Al expirar vuelven a su estado base (idle).
    /// Sin `ttl` = el valor se queda hasta que llegue otro.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u32>,
    /// Parámetros a cambiar. Solo lo que cambia.
    #[serde(default)]
    pub set: Map<String, Value>,
}

/// Valor ya aplanado de un parámetro.
#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    Num(f32),
    Bool(bool),
    Text(String),
}

impl Envelope {
    /// Aplana `set`: los arrays se expanden a `.x .y .z` (`"gaze": [0.3, -0.1]` → `gaze.x`, `gaze.y`).
    /// Los valores no soportados se devuelven como error con su clave.
    pub fn entries(&self) -> impl Iterator<Item = Result<(String, Scalar), String>> + '_ {
        self.set.iter().flat_map(|(k, v)| -> Vec<Result<(String, Scalar), String>> {
            match v {
                Value::Array(items) => items
                    .iter()
                    .zip(["x", "y", "z"])
                    .map(|(item, axis)| scalar(item).map(|s| (format!("{k}.{axis}"), s)).ok_or_else(|| k.clone()))
                    .collect(),
                other => vec![scalar(other).map(|s| (k.clone(), s)).ok_or_else(|| k.clone())],
            }
        })
    }
}

fn scalar(v: &Value) -> Option<Scalar> {
    match v {
        Value::Number(n) => n.as_f64().map(|f| Scalar::Num(f as f32)),
        Value::Bool(b) => Some(Scalar::Bool(*b)),
        Value::String(s) => Some(Scalar::Text(s.clone())),
        _ => None,
    }
}

/// Un frame de red puede traer un sobre o una lista de sobres.
/// Tolera BOM y espacios alrededor (algunas shells/entidades los agregan).
pub fn decode(text: &str) -> Result<Vec<Envelope>, serde_json::Error> {
    let text = text.trim_start_matches('\u{feff}').trim();
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(Envelope),
        Many(Vec<Envelope>),
    }
    Ok(match serde_json::from_str(text)? {
        OneOrMany::One(e) => vec![e],
        OneOrMany::Many(v) => v,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrays_expand_to_axes() {
        let env = &decode(r#"{"s":"face","set":{"gaze":[0.5,-0.25],"expr":"focus"}}"#).unwrap()[0];
        let mut got: Vec<_> = env.entries().map(Result::unwrap).collect();
        got.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            got,
            vec![
                ("expr".into(), Scalar::Text("focus".into())),
                ("gaze.x".into(), Scalar::Num(0.5)),
                ("gaze.y".into(), Scalar::Num(-0.25)),
            ]
        );
    }

    #[test]
    fn batch_decodes() {
        let v = decode(r#"[{"s":"body","set":{}},{"s":"morph","seq":3,"set":{"core":"eyes"}}]"#).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].s, Stream::Morph);
    }

    #[test]
    fn tolerates_bom_and_whitespace() {
        assert_eq!(decode("\u{feff} {\"s\":\"voice\",\"set\":{\"level\":1}}\r\n").unwrap().len(), 1);
    }
}
