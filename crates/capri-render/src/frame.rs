//! Empaqueta el estado del cuerpo en el uniform que lee el shader.
//! El orden de campos DEBE coincidir con `struct Frame` en `shaders/common.wgsl`.

use bytemuck::{Pod, Zeroable};
use capri_core::State;

use crate::style::{hex, Style};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Frame {
    /// ancho, alto, tiempo (s), 1 = salida premultiplicada
    pub res: [f32; 4],
    /// pos.x, pos.y, rot (rad), opacidad global
    pub root: [f32; 4],
    /// escala, yaw (rad), respiración 0..1, parpadeo 0..1
    pub root2: [f32; 4],
    /// peso núcleo full, eyes, bolt, máscara visible
    pub cores: [f32; 4],
    /// ojo izq abierto, ojo der abierto, mirada x, mirada y
    pub eyes: [f32; 4],
    /// brillo máscara, boca abierta (incluye voz), curva boca, tamaño ojo izq
    pub face: [f32; 4],
    /// tamaño ojo der, nivel de voz, -, -
    pub face2: [f32; 4],
    /// cuello, inclinación torso, brazo izq, brazo der (rad)
    pub limbs0: [f32; 4],
    /// pierna izq, pierna der (rad), opacidad silueta, -
    pub limbs1: [f32; 4],
    /// púas, largo púas, fragmentos, dispersión fragmentos
    pub ext: [f32; 4],
    pub col_body: [f32; 4],
    pub col_rim: [f32; 4],
    pub col_mask: [f32; 4],
    pub col_eyes: [f32; 4],
    pub col_hair: [f32; 4],
    pub col_tip: [f32; 4],
    /// glow, densidad scanlines, fuerza scanlines, glitch
    pub fx: [f32; 4],
    /// grosor contorno (px), -, -, -
    pub fx2: [f32; 4],
}

impl Frame {
    pub fn build(state: &State, style: &Style, width: u32, height: u32, premultiplied: bool) -> Self {
        let b = &state.body;
        let (t, f, s, e) = (&b.transform, &b.face, &b.silhouette, &b.extensions);
        let rad = f32::to_radians;
        let voice = f.mouth.voice.value;
        let p = &style.palette;
        Self {
            res: [width as f32, height as f32, state.idle.time(), premultiplied as u8 as f32],
            root: [t.pos_x.value, t.pos_y.value, rad(t.rot.value), t.opacity.value.clamp(0.0, 1.0)],
            root2: [t.scale.value, rad(t.yaw.value), state.idle.breath, state.idle.blink],
            cores: [
                t.shape_morph.full.value.max(0.0),
                t.shape_morph.eyes.value.max(0.0),
                t.shape_morph.bolt.value.max(0.0),
                f.mask.visible.value.clamp(0.0, 1.0),
            ],
            eyes: [f.eye_left.open.value, f.eye_right.open.value, f.gaze_x.value, f.gaze_y.value],
            face: [
                f.mask.glow.value,
                (f.mouth.open.value + voice).clamp(0.0, 1.0),
                f.mouth.curve.value,
                f.eye_left.size.value,
            ],
            face2: [f.eye_right.size.value, voice, 0.0, 0.0],
            limbs0: [rad(s.neck.value), rad(s.torso.lean.value), rad(s.arm_left.value), rad(s.arm_right.value)],
            limbs1: [rad(s.leg_left.value), rad(s.leg_right.value), s.opacity.value.clamp(0.0, 1.0), 0.0],
            ext: [e.spikes.amount.value, e.spikes.length.value, e.fragments.amount.value, e.fragments.spread.value],
            col_body: hex(&p.body),
            col_rim: hex(&p.rim),
            col_mask: hex(&p.mask),
            col_eyes: hex(&p.eyes),
            col_hair: hex(&p.hair),
            col_tip: hex(&p.tip),
            fx: [style.glow, style.scanlines.density, style.scanlines.strength, style.glitch.rate],
            fx2: [style.outline, 0.0, 0.0, 0.0],
        }
    }
}
