//! Pelo procedural de la cabeza.
//!
//! La forma sale de una **plantilla** medida sobre las referencias del personaje (render low-poly
//! de frente, ver `docs/ARCHITECTURE.md`): ~75 triángulos planos con aristas rectas, sin curvas.
//! - **Masa**: un arco (U invertida) de unos 32 R de ancho sobre la placa, con dos lóbulos que
//!   bajan junto a la cara y abrazan la placa casi sin margen. La cara queda en el hueco de abajo.
//! - **Corona**: cuatro aletas de tiburón con las puntas a la misma altura (borde superior plano),
//!   cada una inclinada hacia fuera: arista interior tendida, exterior casi vertical.
//! - **Púas laterales**: hojas finas (vértice ~33°), agujas y algún muñón ancho. Nada apunta en
//!   vertical hacia arriba: el contorno es una pajarita, más largo en las diagonales que arriba.
//!   Algunas llevan un detalle: colmillo (púa que vuelve hacia la raíz), bandera (punta doblada
//!   60–90°) o escalón.
//! - **Capa delantera**: flequillo y mechones que van delante de la cara, todos barridos hacia
//!   abajo a la izquierda (~-130°). Dos puntas bajan hasta la placa: una sobre el anillo del ojo
//!   izquierdo y otra entre los ojos. Los ojos y el 60 % de abajo de la placa quedan libres.
//!
//! Cada **tramo** de giro (yaw, pitch o roll) mueve un poco los vértices al azar (el mismo tramo
//! da siempre el mismo pelo); el tramo central es la plantilla exacta. Entre umbrales no se
//! recalcula nada: solo se sube a la GPU al cambiar de tramo.
//!
//! Unidades: R (radio del anillo del ojo, como `face.wgsl`), origen en el centro de la cabeza
//! (= centro de la placa), y hacia arriba. Las posiciones salen ya giradas con los ángulos del
//! tramo; el shader solo sigue el resto continuo del roll.

use bytemuck::{Pod, Zeroable};
use std::f32::consts::TAU;

pub const MAX_SHARDS: usize = 112;
/// Umbrales de cada eje, en grados. El tramo central es |ángulo| < paso / 2.
pub const YAW_STEP: f32 = 20.0;
pub const PITCH_STEP: f32 = 15.0;
pub const ROLL_STEP: f32 = 15.0;
/// Margen para dejar un tramo: evita que el pelo salte de forma en el borde de un umbral.
const HYSTERESIS: f32 = 1.5;
/// Radio de la esfera de la cabeza (= `HEAD_R` de `face.wgsl`).
const HEAD_R: f32 = 5.4;
/// Profundidad del centro de la masa: detrás del centro de la cabeza, así al girar se corre al
/// lado contrario de la cara.
const MASS_Z: f32 = -2.2;
/// Profundidad del plano de la capa delantera: sobre la frente, se mueve con la cara.
const FRONT_Z: f32 = 0.8 * HEAD_R;
/// Centro de la masa en la plantilla.
const MASS_CENTER: V2 = [0.0, 7.5];

/// Uniform del pelo. El orden DEBE coincidir con `struct Hair` en `shaders/hair.wgsl`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HairData {
    /// triángulos, roll del tramo (rad), alcance (fuera de él no hay pelo), -
    pub info: [f32; 4],
    /// foco de las manchas (x, y), radio del foco, semilla de las manchas
    pub glint: [f32; 4],
    /// dos vec4 por triángulo: (a.x, a.y, b.x, b.y), (c.x, c.y, tono, capa: 0 detrás, 1 delante)
    pub shards: [[f32; 4]; MAX_SHARDS * 2],
}

/// Tramo actual + su pelo generado.
pub struct Hair {
    bucket: Option<[i32; 3]>,
    data: HairData,
}

impl Default for Hair {
    fn default() -> Self {
        Self { bucket: None, data: HairData::zeroed() }
    }
}

impl Hair {
    /// Ángulos actuales en grados. Devuelve el pelo nuevo solo si cambió de tramo.
    pub fn update(&mut self, yaw: f32, pitch: f32, roll: f32) -> Option<&HairData> {
        let angles = [(yaw + 180.0).rem_euclid(360.0) - 180.0, pitch, roll];
        let steps = [YAW_STEP, PITCH_STEP, ROLL_STEP];
        let next = std::array::from_fn(|i| bucket(angles[i], steps[i], self.bucket.map(|b| b[i])));
        if self.bucket == Some(next) {
            return None;
        }
        self.bucket = Some(next);
        self.data = build(next).data;
        Some(&self.data)
    }
}

fn bucket(angle: f32, step: f32, current: Option<i32>) -> i32 {
    match current {
        Some(c) if (angle - c as f32 * step).abs() <= step * 0.5 + HYSTERESIS => c,
        _ => (angle / step).round() as i32,
    }
}

type V2 = [f32; 2];

fn add(a: V2, b: V2) -> V2 {
    [a[0] + b[0], a[1] + b[1]]
}

fn rot(v: V2, (s, c): (f32, f32)) -> V2 {
    [c * v[0] - s * v[1], s * v[0] + c * v[1]]
}

/// Giro de la cabeza (igual que `head_inverse` de `face.wgsl`, en sentido directo):
/// cabeza → pantalla.
struct Turn {
    yaw: (f32, f32),
    pitch: (f32, f32),
    roll: (f32, f32),
}

impl Turn {
    fn new(yaw: f32, pitch: f32, roll: f32) -> Self {
        Self { yaw: yaw.sin_cos(), pitch: pitch.sin_cos(), roll: roll.sin_cos() }
    }

    fn project(&self, [x, y, z]: [f32; 3]) -> V2 {
        let ((sr, cr), (sp, cp), (sy, cy)) = (self.roll, self.pitch, self.yaw);
        let (x, y) = (cr * x - sr * y, sr * x + cr * y);
        let (y, z) = (cp * y + sp * z, -sp * y + cp * z);
        [cy * x + sy * z, y]
    }

    /// Cuánto mira la cara hacia el espectador: 1 de frente, 0 de perfil, -1 de espalda.
    fn facing(&self) -> f32 {
        self.yaw.1 * self.pitch.1
    }
}

/// xorshift32 sembrado con el tramo: el mismo tramo da siempre el mismo pelo.
struct Rng(u32);

impl Rng {
    fn new(b: [i32; 3], salt: u32) -> Self {
        let mut h = 0x811C_9DC5u32;
        for v in [b[0], b[1], b[2], salt as i32] {
            h = (h ^ v as u32).wrapping_mul(0x0100_0193);
        }
        let mut rng = Self(h | 1);
        for _ in 0..4 {
            rng.next();
        }
        rng
    }

    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next()
    }
}

/// Capa de un triángulo de la plantilla.
#[derive(Clone, Copy, PartialEq)]
enum Layer {
    /// Detrás de la cara.
    Back,
    /// Delante de la cara (flequillo, mechones sobre la placa).
    Front,
    /// Detrás de la placa, en su mismo plano: calza con la placa gire como gire.
    Fill,
    /// Tapa el hueco de la cara en la masa: va con ella, así gire como gire no queda espacio
    /// entre la masa y el relleno de la placa.
    Bridge,
    /// Solo de espalda: tapa el hueco de la cara.
    Nape,
}

/// Cuánto puede moverse un vértice de la plantilla entre tramos.
const JITTER_FIXED: f32 = 0.15; // bordes del hueco de la cara y puntas del flequillo
const JITTER_CORE: f32 = 0.4; // vértices de la masa
const JITTER_TIP: f32 = 0.7; // puntas y detalles de las púas

/// Vértices que casi no se mueven: el hueco abraza la placa y las puntas del flequillo caen
/// sobre el anillo del ojo izquierdo y entre los ojos.
const FIXED: [V2; 7] = [[-2.0, 2.4], [4.6, 2.7], [-4.1, -1.4], [-4.8, -0.5], [4.7, -0.7], [-2.3, 0.9], [-0.7, 0.6]];

/// Masa: tres polígonos estrella abiertos en abanico desde su centro.
const CORE_CROWN: [V2; 15] = [
    [-15.5, 12.4], [-14.4, 10.4], [-14.3, 7.8], [-13.0, 7.4], [-15.9, 5.7], [-15.0, 1.5], [-2.0, 2.4],
    [4.6, 2.7], [16.0, 4.0], [10.7, 7.0], [14.3, 11.2], [13.0, 12.9], [5.8, 12.8], [0.4, 11.8], [-7.4, 11.7],
];
const CORE_LEFT: [V2; 9] = [
    [-15.0, 1.5], [-16.5, -1.6], [-13.0, -2.7], [-10.5, -3.3], [-7.6, -2.9], [-4.8, -2.4], [-4.1, -1.4],
    [-4.8, -0.5], [-2.0, 2.4],
];
const CORE_RIGHT: [V2; 13] = [
    [4.6, 2.7], [4.7, -0.7], [4.0, -2.5], [6.9, -2.3], [8.1, -1.8], [8.2, -2.9], [12.5, -4.5], [14.7, -4.2],
    [17.7, -2.7], [19.2, -1.2], [18.9, 0.9], [16.6, 1.2], [16.0, 4.0],
];

/// Relleno detrás de la placa (media placa = 4.04 × 1.654), en el plano de la placa.
const FILL: [V2; 9] = [
    [-6.5, 5.0], [6.5, 5.0], [6.5, -2.5], [4.6, -2.3], [2.6, -2.05], [0.0, -2.4], [-2.4, -2.0], [-4.6, -2.4],
    [-6.5, -2.2],
];
/// Tapa del hueco de la masa (sube bajo ella y baja hasta el borde de los lóbulos).
const BRIDGE: [V2; 11] = [
    [-11.0, 9.0], [11.0, 9.0], [11.0, -1.8], [6.5, -2.2], [4.6, -2.3], [2.6, -2.05], [0.0, -2.4], [-2.4, -2.0],
    [-4.6, -2.4], [-6.5, -2.2], [-11.0, -1.8],
];

struct Template {
    tris: Vec<([V2; 3], Layer)>,
}

impl Template {
    fn tri(&mut self, a: V2, b: V2, c: V2) {
        self.tris.push(([a, b, c], Layer::Back));
    }

    fn front(&mut self, a: V2, b: V2, c: V2) {
        self.tris.push(([a, b, c], Layer::Front));
    }

    /// Polígono cerrado en abanico desde `c`.
    fn star(&mut self, c: V2, poly: &[V2], layer: Layer) {
        for i in 0..poly.len() {
            self.tris.push(([c, poly[i], poly[(i + 1) % poly.len()]], layer));
        }
    }

    /// Polígono convexo en abanico desde su primer vértice.
    fn fan(&mut self, poly: &[V2]) {
        for i in 1..poly.len() - 1 {
            self.tri(poly[0], poly[i], poly[i + 1]);
        }
    }
}

/// La plantilla: el pelo de frente, en el espacio de la placa.
fn template() -> Template {
    let mut t = Template { tris: Vec::with_capacity(MAX_SHARDS) };
    // Detrás de la cara nunca queda espacio: un relleno pegado a la placa y una tapa del hueco de
    // la masa. Van primero, así las facetas de la masa quedan encima y no se notan.
    t.star([0.0, 1.0], &FILL, Layer::Fill);
    t.star([0.0, 1.0], &BRIDGE, Layer::Bridge);
    t.star(MASS_CENTER, &CORE_CROWN, Layer::Back);
    t.star([-9.0, -0.5], &CORE_LEFT, Layer::Back);
    t.star([11.0, 0.0], &CORE_RIGHT, Layer::Back);

    // Corona: aletas con la punta a y ≈ 15.5, inclinadas hacia fuera.
    t.tri([-7.4, 11.7], [-15.5, 12.4], [-16.9, 15.6]);
    t.fan(&[[-16.9, 15.6], [-15.2, 15.6], [-8.3, 13.8], [-7.4, 11.7]]); // punta truncada, plana
    t.tri([-16.9, 15.6], [-15.5, 12.4], [-15.6, 10.2]); // colmillo: vuelve hacia la raíz
    t.tri([0.4, 11.8], [-7.4, 11.7], [-7.8, 15.3]);
    t.fan(&[[0.4, 11.8], [5.8, 12.8], [5.5, 15.6], [2.9, 14.4], [2.5, 13.3]]); // con escalón
    t.tri([5.8, 12.8], [14.3, 11.2], [16.0, 15.5]);

    // Lado izquierdo.
    t.tri([-19.1, 6.2], [-16.8, 5.9], [-20.2, 9.7]);
    t.fan(&[[-16.0, 5.7], [-19.1, 6.2], [-20.7, 5.9], [-18.0, 1.8], [-16.5, 1.2], [-15.0, 1.5]]); // muñón
    t.tri([-15.3, 1.8], [-21.0, -3.6], [-16.5, -1.6]);
    t.tri([-14.1, -1.1], [-16.1, -4.3], [-12.8, -5.5]); // esquirla suelta
    t.tri([-12.4, -3.0], [-10.5, -3.3], [-13.3, -7.3]);

    // Lado derecho.
    t.tri([10.7, 7.0], [15.6, 8.8], [16.0, 4.0]);
    t.tri([15.7, 5.1], [16.8, 6.1], [15.6, 8.8]); // tallo...
    t.tri([15.6, 8.8], [16.8, 6.1], [17.1, 10.6]);
    t.tri([15.6, 8.8], [17.1, 10.6], [17.1, 11.7]);
    t.tri([17.1, 10.6], [18.8, 11.6], [17.1, 11.7]); // ...con bandera
    t.tri([16.4, 4.6], [18.2, 3.3], [19.9, 10.1]);
    t.fan(&[[16.0, 4.0], [16.6, 1.2], [18.2, 3.3], [16.4, 4.6]]);
    t.tri([16.6, 1.2], [18.9, 0.9], [20.3, 3.2]);
    t.tri([17.4, -3.7], [19.9, -4.3], [20.2, -3.1]);
    t.tri([17.4, -3.7], [20.2, -3.1], [18.6, -0.8]);
    t.tri([19.2, -1.2], [19.9, -0.4], [18.9, 0.9]);
    t.tri([14.7, -4.2], [14.3, -6.4], [17.7, -2.7]);

    // Delante de la cara: todo barrido hacia abajo a la izquierda.
    t.front([-1.6, 2.6], [-8.5, 1.5], [-7.6, -3.7]); // mechón doblado: pared izquierda del hueco
    t.front([-1.6, 2.6], [-7.6, -3.7], [-4.8, -0.5]);
    t.front([4.0, -2.5], [8.2, -5.95], [6.6, -2.6]);
    t.front([6.3, -3.8], [8.5, -3.8], [6.9, -2.3]);
    t.front([5.6, -0.6], [0.7, -4.0], [4.6, -2.0]); // aguja bajo la barbilla
    t.front([4.9, 3.2], [3.75, 1.0], [4.8, 0.6]); // punta sobre la esquina de la placa
    t.front([0.4, 5.0], [-2.3, 0.9], [1.4, 4.4]); // flequillo: sobre el ojo izquierdo
    t.front([1.4, 4.0], [-0.7, 0.6], [2.4, 3.3]); // flequillo: entre los ojos

    // Nuca: de espalda el pelo tapa toda la cabeza.
    let nape: Vec<V2> = (0..10).map(|k| rot([HEAD_R + 1.0, 0.0], (TAU * k as f32 / 10.0).sin_cos())).collect();
    t.star([0.0, 0.0], &nape, Layer::Nape);
    t
}

/// Desplazamiento fijo de un vértice de la plantilla en este tramo: depende solo de su posición,
/// así los vértices compartidos se mueven juntos y no se abren rendijas.
fn jitter(b: [i32; 3], v: V2) -> V2 {
    let amp = if FIXED.contains(&v) {
        JITTER_FIXED
    } else if CORE_CROWN.contains(&v) || CORE_LEFT.contains(&v) || CORE_RIGHT.contains(&v) {
        JITTER_CORE
    } else {
        JITTER_TIP
    };
    let key = ((v[0] * 20.0).round() as i32 as u32).wrapping_mul(73_856_093) ^ ((v[1] * 20.0).round() as i32 as u32);
    let mut rng = Rng::new(b, key);
    [amp * rng.range(-1.0, 1.0), amp * rng.range(-1.0, 1.0)]
}

#[cfg_attr(not(test), allow(dead_code))] // `center` y `tris` solo los leen los tests
struct Built {
    data: HairData,
    /// centro de la masa en pantalla
    center: V2,
    /// triángulos en pantalla y si van delante de la cara
    tris: Vec<([V2; 3], bool)>,
}

fn build(b: [i32; 3]) -> Built {
    let (yaw, pitch, roll) = (
        (b[0] as f32 * YAW_STEP).to_radians(),
        (b[1] as f32 * PITCH_STEP).to_radians(),
        (b[2] as f32 * ROLL_STEP).to_radians(),
    );
    let turn = Turn::new(yaw, pitch, roll);
    let facing = turn.facing();
    let mut rng = Rng::new(b, 0);
    // El tramo central es la plantilla exacta.
    let shake = if b == [0, 0, 0] { 0.0 } else { 1.0 };

    // La masa se corre con el giro (vive detrás de la cabeza) y se estrecha un poco de perfil;
    // de espalda se ve en espejo. La capa delantera va pegada a la cara.
    let mirror = if facing < 0.0 { -1.0 } else { 1.0 };
    let (sx, sy) = (1.0 - 0.25 * turn.yaw.0.abs(), 1.0 - 0.15 * turn.pitch.0.abs());
    let offset = turn.project([0.0, 0.0, MASS_Z]);
    let back = |v: V2| add(rot([mirror * v[0] * sx, v[1] * sy], turn.roll), offset);
    let front = |v: V2| turn.project([v[0], v[1], FRONT_Z]);
    // El flequillo solo mientras la cara mira al frente; la nuca en cuanto deja de mirarlo.
    let show_front = facing > 0.6;
    let show_nape = facing < 0.5;

    let mut data = HairData::zeroed();
    let mut tris = Vec::with_capacity(MAX_SHARDS);
    for (verts, layer) in template().tris {
        let in_front = layer == Layer::Front && show_front;
        if layer == Layer::Nape && !show_nape {
            continue;
        }
        let [p0, p1, p2] = verts.map(|v| {
            let j = if matches!(layer, Layer::Fill | Layer::Bridge) { [0.0; 2] } else { jitter(b, v) };
            let v = [v[0] + shake * j[0], v[1] + shake * j[1]];
            match layer {
                Layer::Fill => front(v),
                Layer::Nape => rot(v, turn.roll), // centrada en la cabeza, no en la masa
                _ if in_front => front(v),
                _ => back(v),
            }
        });
        let n = tris.len();
        if n == MAX_SHARDS {
            break;
        }
        data.shards[2 * n] = [p0[0], p0[1], p1[0], p1[1]];
        data.shards[2 * n + 1] = [p2[0], p2[1], rng.range(0.82, 1.0), if in_front { 1.0 } else { 0.0 }];
        tris.push(([p0, p1, p2], in_front));
    }

    // Manchas: en el tramo central se concentran en el centro del pelo; en los demás, hacia un
    // punto cardinal del plano del pelo, casi siempre arriba a la derecha y nunca abajo.
    let center = back(MASS_CENTER);
    let (focus, radius) = if b == [0, 0, 0] {
        (add(center, rot([0.0, 0.5], turn.roll)), 7.5)
    } else {
        let deg = (45.0 - 12.0 * b[0] as f32 + 15.0 * b[1] as f32 + rng.range(-12.0, 12.0)).clamp(10.0, 170.0);
        (add(center, rot(rot([8.0, 0.0], deg.to_radians().sin_cos()), turn.roll)), 9.0)
    };
    let reach = tris.iter().flat_map(|(t, _)| t.iter()).map(|p| p[0].hypot(p[1])).fold(0.0, f32::max);

    data.info = [tris.len() as f32, roll, reach + 1.0, 0.0];
    data.glint = [focus[0], focus[1], radius, rng.range(0.0, 97.0)];
    Built { data, center, tris }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buckets() -> impl Iterator<Item = [i32; 3]> {
        (-9..=9).flat_map(|y| (-4..=4).flat_map(move |p| (-3..=3).map(move |r| [y, p, r])))
    }

    fn inside(p: V2, [a, b, c]: [V2; 3]) -> bool {
        let cross = |o: V2, e: V2| (e[0] - o[0]) * (p[1] - o[1]) - (e[1] - o[1]) * (p[0] - o[0]);
        let (d0, d1, d2) = (cross(a, b), cross(b, c), cross(c, a));
        // con tolerancia: un punto sobre la arista compartida de dos triángulos cuenta
        let e = 1e-3;
        (d0 >= -e && d1 >= -e && d2 >= -e) || (d0 <= e && d1 <= e && d2 <= e)
    }

    #[test]
    fn regenerates_only_when_crossing_a_threshold() {
        let mut hair = Hair::default();
        assert!(hair.update(0.0, 0.0, 0.0).is_some(), "el primer frame genera el pelo");
        assert!(hair.update(9.0, 5.0, -7.0).is_none(), "dentro del tramo central no cambia");
        assert!(hair.update(11.0, 5.0, -7.0).is_none(), "la histéresis retiene el tramo en el borde");
        assert!(hair.update(12.0, 5.0, -7.0).is_some(), "pasado el margen cambia de forma");
        assert!(hair.update(9.0, 5.0, -7.0).is_none(), "volver un poco no lo regresa");
        assert!(hair.update(8.0, 5.0, -7.0).is_some());
        assert!(hair.update(0.0, 0.0, 20.0).is_some(), "el roll también tiene umbrales");
    }

    #[test]
    fn same_bucket_same_hair() {
        let (a, b, c) = (build([1, -1, 0]).data, build([1, -1, 0]).data, build([2, -1, 0]).data);
        assert_eq!(bytemuck::bytes_of(&a), bytemuck::bytes_of(&b));
        assert_ne!(bytemuck::bytes_of(&a), bytemuck::bytes_of(&c));
    }

    #[test]
    fn fits_the_uniform_in_every_bucket() {
        assert!(template().tris.len() <= MAX_SHARDS);
        for b in buckets() {
            let h = build(b);
            assert!(h.data.info[0] as usize <= MAX_SHARDS, "{b:?}");
        }
    }

    #[test]
    fn from_the_front_the_eyes_stay_visible() {
        // Centro de cada ojo y bajo cada uno: nada de la capa delantera los tapa
        // (la de detrás queda bajo la placa).
        let free: [V2; 5] = [[-2.03, -0.365], [2.03, -0.365], [-2.03, -1.3], [0.0, -1.3], [2.03, -1.3]];
        for b in buckets().filter(|b| b[0].abs() <= 1 && b[1].abs() <= 1) {
            let h = build(b);
            let turn = Turn::new(
                (b[0] as f32 * YAW_STEP).to_radians(),
                (b[1] as f32 * PITCH_STEP).to_radians(),
                (b[2] as f32 * ROLL_STEP).to_radians(),
            );
            for [x, y] in free {
                // la placa va sobre la esfera de la cabeza
                let p = turn.project([x, y, (HEAD_R * HEAD_R - x * x - y * y).sqrt()]);
                assert!(!h.tris.iter().any(|&(t, front)| front && inside(p, t)), "{b:?}: tapa {x}, {y}");
            }
        }
    }

    #[test]
    fn behind_the_plate_there_is_always_hair() {
        // Alrededor y detrás de la placa (con margen) no queda ningún hueco, gire como gire.
        for b in buckets().filter(|b| b[0].abs() <= 2 && b[1].abs() <= 2) {
            let h = build(b);
            let turn = Turn::new(
                (b[0] as f32 * YAW_STEP).to_radians(),
                (b[1] as f32 * PITCH_STEP).to_radians(),
                (b[2] as f32 * ROLL_STEP).to_radians(),
            );
            for i in 0..=12 {
                for j in 0..=6 {
                    let (x, y) = (-4.4 + 8.8 * i as f32 / 12.0, -1.9 + 3.8 * j as f32 / 6.0);
                    let p = turn.project([x, y, FRONT_Z]);
                    assert!(h.tris.iter().any(|&(t, _)| inside(p, t)), "{b:?}: hueco en {x:.1}, {y:.1}");
                }
            }
        }
    }

    #[test]
    fn the_bangs_reach_the_plate() {
        let h = build([0, 0, 0]);
        let bangs = h.tris.iter().filter(|(_, front)| *front).count();
        assert!(bangs >= 6, "flequillo delante de la cara ({bangs})");
        assert!(h.tris.iter().any(|&(t, front)| front && inside([-0.3, 1.2], t)), "punta entre los ojos");
    }

    #[test]
    fn from_behind_the_hair_covers_the_head() {
        for b in [[9, 0, 0], [-9, 0, 0], [5, 0, 0], [-5, 0, 0]] {
            let h = build(b);
            for k in 0..12 {
                let p = rot([0.8 * HEAD_R, 0.0], (TAU * k as f32 / 12.0).sin_cos());
                assert!(h.tris.iter().any(|&(t, _)| inside(p, t)), "{b:?}: cabeza al aire en {p:?}");
            }
        }
    }

    #[test]
    fn glints_center_in_the_middle_bucket_and_never_below_elsewhere() {
        let mid = build([0, 0, 0]);
        let off = (mid.data.glint[0] - mid.center[0]).hypot(mid.data.glint[1] - mid.center[1]);
        assert!(off < 1.5, "tramo central: foco en el centro del pelo ({off})");
        for b in buckets().filter(|b| b[2] == 0 && *b != [0, 0, 0]) {
            let h = build(b);
            assert!(h.data.glint[1] > h.center[1], "{b:?}: el foco no puede quedar abajo");
        }
    }
}
