//! ENTITY_BODY: única fuente de verdad de la anatomía.
//!
//! ```text
//! ENTITY_BODY
//! ├── TransformController   transform global + núcleos (ShapeMorph)
//! ├── Face                  Mask, EyeLeft, EyeRight, Mouth
//! ├── Silhouette            Torso, Arms, Legs
//! └── Extensions            Spikes, Fragments
//! ```
//!
//! Cada rama declara sus parámetros con `visit`: stream dueño + clave + parámetro.
//! Una clave tiene UN solo stream dueño, así dos streams nunca se pelean.

mod extensions;
mod face;
mod silhouette;
mod transform;

pub use extensions::*;
pub use face::*;
pub use silhouette::*;
pub use transform::*;

use crate::Param;
use capri_protocol::Stream;

/// Recorre los parámetros de un nodo: `(stream dueño, clave, parámetro)`.
pub trait Node {
    fn visit(&mut self, f: &mut dyn FnMut(Stream, &'static str, &mut Param));
}

#[derive(Debug, Clone, Default)]
pub struct EntityBody {
    pub transform: TransformController,
    pub face: Face,
    pub silhouette: Silhouette,
    pub extensions: Extensions,
}

impl Node for EntityBody {
    fn visit(&mut self, f: &mut dyn FnMut(Stream, &'static str, &mut Param)) {
        self.transform.visit(f);
        self.face.visit(f);
        self.silhouette.visit(f);
        self.extensions.visit(f);
    }
}

impl EntityBody {
    /// Aplica `f` al parámetro `key` del `stream` dueño. Devuelve false si no existe.
    pub fn with_param(&mut self, stream: Stream, key: &str, f: impl FnOnce(&mut Param)) -> bool {
        let mut f = Some(f);
        self.visit(&mut |owner, k, p| {
            if owner == stream && k == key {
                if let Some(f) = f.take() {
                    f(p);
                }
            }
        });
        f.is_none()
    }

    /// Todas las claves que acepta un stream (para mensajes de error y docs).
    pub fn keys(stream: Stream) -> Vec<&'static str> {
        let mut out = Vec::new();
        EntityBody::default().visit(&mut |owner, k, _| {
            if owner == stream {
                out.push(k);
            }
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_key_has_a_single_owner() {
        let mut keys = HashSet::new();
        EntityBody::default().visit(&mut |_, k, _| {
            assert!(keys.insert(k), "clave repetida o con dos dueños: {k}");
        });
    }
}
