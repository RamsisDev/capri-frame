//! El cuerpo como datos. Los streams escriben aquí; el render solo lee.

pub mod body;
pub mod idle;
pub mod param;
pub mod presets;
pub mod state;

pub use body::EntityBody;
pub use param::Param;
pub use state::State;
