//! Render de Capri. Solo LEE el estado del cuerpo; nunca ve la red.
//!
//! Un solo draw call a pantalla completa. Las "capas" están separadas en módulos WGSL
//! que siguen el árbol ENTITY_BODY (silhouette, extensions, face) + fx, y se componen
//! en `main.wgsl`. Se pueden partir en passes reales el día que haga falta.

mod frame;
mod renderer;
pub mod style;

pub use frame::Frame;
pub use renderer::Renderer;
pub use style::{Style, StyleFile};
