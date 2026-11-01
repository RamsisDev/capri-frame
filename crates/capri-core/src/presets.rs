//! Presets probados: la entidad puede mandar un nombre en vez de valores crudos.
//! Un preset solo se expande a claves normales; no hay lógica especial detrás.
//! Las claves que el preset no toca vuelven a su valor neutral cuando se listan aquí.

type Preset = &'static [(&'static str, f32)];

/// `face.expr`
pub fn expr(name: &str) -> Option<Preset> {
    Some(match name {
        "neutral" => &[("eye.l.open", 1.0), ("eye.r.open", 1.0), ("eye.l.size", 1.0), ("eye.r.size", 1.0),
                       ("mouth.open", 0.0), ("mouth.curve", 0.0), ("eye.lid", 0.0), ("mask.glow", 0.6)],
        "focus" => &[("eye.l.open", 0.55), ("eye.r.open", 0.55), ("eye.l.size", 0.9), ("eye.r.size", 0.9),
                     ("mouth.open", 0.0), ("mouth.curve", 0.0), ("eye.lid", 0.0), ("mask.glow", 0.8)],
        "happy" => &[("eye.l.open", 0.35), ("eye.r.open", 0.35), ("eye.l.size", 1.0), ("eye.r.size", 1.0),
                     ("mouth.open", 0.2), ("mouth.curve", 1.0), ("eye.lid", 1.0), ("mask.glow", 0.9)],
        "angry" => &[("eye.l.open", 0.45), ("eye.r.open", 0.45), ("eye.l.size", 0.8), ("eye.r.size", 0.8),
                     ("mouth.open", 0.1), ("mouth.curve", -0.8), ("eye.lid", 0.0), ("mask.glow", 1.0)],
        "sad" => &[("eye.l.open", 0.6), ("eye.r.open", 0.6), ("eye.l.size", 1.0), ("eye.r.size", 1.0),
                   ("mouth.open", 0.0), ("mouth.curve", -1.0), ("eye.lid", -1.0), ("mask.glow", 0.3)],
        "surprised" => &[("eye.l.open", 1.0), ("eye.r.open", 1.0), ("eye.l.size", 1.35), ("eye.r.size", 1.35),
                         ("mouth.open", 0.8), ("mouth.curve", 0.0), ("eye.lid", 0.0), ("mask.glow", 1.0)],
        "sleep" => &[("eye.l.open", 0.0), ("eye.r.open", 0.0), ("eye.l.size", 1.0), ("eye.r.size", 1.0),
                     ("mouth.open", 0.0), ("mouth.curve", 0.0), ("eye.lid", 0.0), ("mask.glow", 0.15)],
        _ => return None,
    })
}

/// `voice.viseme`: una forma de boca entera; el resto de pesos vuelve a 0.
/// Acepta el nombre en minúsculas o mayúsculas (`"A"`, `"mbp"`, `"F/L"` o `"fl"`).
pub fn viseme(name: &str) -> Option<Preset> {
    const ALL: [Preset; 8] = [
        &[("viseme.x", 1.0), ("viseme.mbp", 0.0), ("viseme.a", 0.0), ("viseme.e", 0.0), ("viseme.i", 0.0), ("viseme.o", 0.0), ("viseme.u", 0.0), ("viseme.fl", 0.0)],
        &[("viseme.x", 0.0), ("viseme.mbp", 1.0), ("viseme.a", 0.0), ("viseme.e", 0.0), ("viseme.i", 0.0), ("viseme.o", 0.0), ("viseme.u", 0.0), ("viseme.fl", 0.0)],
        &[("viseme.x", 0.0), ("viseme.mbp", 0.0), ("viseme.a", 1.0), ("viseme.e", 0.0), ("viseme.i", 0.0), ("viseme.o", 0.0), ("viseme.u", 0.0), ("viseme.fl", 0.0)],
        &[("viseme.x", 0.0), ("viseme.mbp", 0.0), ("viseme.a", 0.0), ("viseme.e", 1.0), ("viseme.i", 0.0), ("viseme.o", 0.0), ("viseme.u", 0.0), ("viseme.fl", 0.0)],
        &[("viseme.x", 0.0), ("viseme.mbp", 0.0), ("viseme.a", 0.0), ("viseme.e", 0.0), ("viseme.i", 1.0), ("viseme.o", 0.0), ("viseme.u", 0.0), ("viseme.fl", 0.0)],
        &[("viseme.x", 0.0), ("viseme.mbp", 0.0), ("viseme.a", 0.0), ("viseme.e", 0.0), ("viseme.i", 0.0), ("viseme.o", 1.0), ("viseme.u", 0.0), ("viseme.fl", 0.0)],
        &[("viseme.x", 0.0), ("viseme.mbp", 0.0), ("viseme.a", 0.0), ("viseme.e", 0.0), ("viseme.i", 0.0), ("viseme.o", 0.0), ("viseme.u", 1.0), ("viseme.fl", 0.0)],
        &[("viseme.x", 0.0), ("viseme.mbp", 0.0), ("viseme.a", 0.0), ("viseme.e", 0.0), ("viseme.i", 0.0), ("viseme.o", 0.0), ("viseme.u", 0.0), ("viseme.fl", 1.0)],
    ];
    let key = name.to_ascii_lowercase().replace('/', "");
    crate::body::VISEMES.iter().position(|v| *v == key).map(|i| ALL[i])
}

/// `body.pose`
pub fn pose(name: &str) -> Option<Preset> {
    Some(match name {
        "idle" => &[("neck", 0.0), ("torso.lean", 0.0), ("arm.l", 0.0), ("arm.r", 0.0), ("leg.l", 0.0), ("leg.r", 0.0)],
        "arms_up" => &[("neck", 0.0), ("torso.lean", 0.0), ("arm.l", -150.0), ("arm.r", 150.0), ("leg.l", 0.0), ("leg.r", 0.0)],
        "wave" => &[("neck", 8.0), ("torso.lean", 4.0), ("arm.l", 0.0), ("arm.r", 130.0), ("leg.l", 0.0), ("leg.r", 0.0)],
        "guard" => &[("neck", -6.0), ("torso.lean", -5.0), ("arm.l", -60.0), ("arm.r", 60.0), ("leg.l", -12.0), ("leg.r", 12.0)],
        "lean_left" => &[("neck", 10.0), ("torso.lean", 12.0), ("arm.l", -15.0), ("arm.r", 10.0), ("leg.l", 0.0), ("leg.r", 8.0)],
        "lean_right" => &[("neck", -10.0), ("torso.lean", -12.0), ("arm.l", -10.0), ("arm.r", 15.0), ("leg.l", -8.0), ("leg.r", 0.0)],
        _ => return None,
    })
}

/// `morph.core`: los 3 núcleos.
/// - `full`: cuerpo completo.
/// - `eyes`: solo los ojos (con su placa).
/// - `bolt`: rayo fragmentado.
pub fn core(name: &str) -> Option<Preset> {
    Some(match name {
        "full" => &[("core.full", 1.0), ("core.eyes", 0.0), ("core.bolt", 0.0), ("silhouette", 1.0),
                    ("spikes", 1.0), ("fragments", 0.0)],
        "eyes" => &[("core.full", 0.0), ("core.eyes", 1.0), ("core.bolt", 0.0), ("silhouette", 0.0),
                    ("spikes", 0.0), ("fragments", 0.0)],
        "bolt" => &[("core.full", 0.0), ("core.eyes", 0.0), ("core.bolt", 1.0), ("silhouette", 1.0),
                    ("spikes", 0.0), ("fragments", 0.7)],
        _ => return None,
    })
}
