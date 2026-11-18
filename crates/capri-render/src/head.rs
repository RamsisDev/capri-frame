//! Cara oscura de la cabeza: el sólido negro que lleva la placa, con los cuernos laterales y la
//! barbilla del refsheet del personaje (`docs/ARCHITECTURE.md`).
//!
//! La forma es una **unión de piezas convexas en 3D** medidas sobre las diez vistas del refsheet
//! (yaw −90..90 y pitch −45..45). La proyección de un casco convexo es el casco convexo de sus
//! puntos proyectados, así que cada frame basta con proyectar los puntos de cada pieza, tomar su
//! casco 2D. La silueta es la unión de esos polígonos convexos (un solo color, sin orden de
//! profundidad). Las piezas se solapan en 3D, así que en pantalla no quedan costuras entre ellas.
//! Solo se recalcula y se sube a la GPU cuando cambia el giro.
//!
//! Unidades: R (radio del anillo del ojo, como `face.wgsl`), origen en el centro de la cabeza
//! (= centro de la placa), y hacia arriba, z hacia el espectador (la cara mira a +z).

use bytemuck::{Pod, Zeroable};

use crate::hair::{Turn, V2};

pub const MAX_PARTS: usize = 16;
/// Vértices de todos los polígonos juntos (dos por vec4).
pub const MAX_VERTS: usize = 128;

/// Uniform de la cara. El orden DEBE coincidir con `struct Head` en `shaders/head.wgsl`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HeadData {
    /// piezas, alcance (fuera de él no hay cara), -, -
    pub info: [f32; 4],
    /// dos vec4 por pieza: (primer vértice, vértices, -, -), caja (lo.x, lo.y, hi.x, hi.y)
    pub parts: [[f32; 4]; MAX_PARTS * 2],
    /// vértices de los polígonos en sentido antihorario, dos por vec4
    pub verts: [[f32; 4]; MAX_VERTS / 2],
}

/// (nombre de la pieza, puntos). Cada pieza es el casco convexo de sus puntos; ya espejadas en x.
/// Medidas sobre el refsheet con la máscara verde de frente = ancho de la placa (17.45 px/R):
/// IoU 0.96 de frente, 0.88 a ±45° y 0.89 de perfil. Contienen la placa sobre la esfera y la boca
/// abierta con margen. La cara es un plano en z ≈ 5.7, por delante de la esfera.
const PARTS: &[(&str, &[[f32; 3]])] = &[
    // Cráneo con la cara plana delante.
    ("core", &[[0.89, 2.81, 4.56], [4.83, 1.91, 5.68], [6.56, 1.18, 3.75], [4.05, -3.30, 2.55], [2.13, -3.44, 6.03], [3.81, 2.17, -2.36], [6.24, -1.64, -2.60], [-0.89, 2.81, 4.56], [-4.83, 1.91, 5.68], [-6.56, 1.18, 3.75], [-4.05, -3.30, 2.55], [-2.13, -3.44, 6.03], [-3.81, 2.17, -2.36], [-6.24, -1.64, -2.60]]),
    // Mandíbula adelantada (1.6 R por delante de la placa) con la barbilla en punta.
    ("jaw", &[[0.00, -2.87, 7.03], [0.00, -6.46, 5.65], [4.06, -0.74, 5.32], [5.17, -3.50, 3.17], [1.36, -6.05, 1.65], [1.84, -5.86, -0.19], [0.15, -2.98, -1.94], [-4.06, -0.74, 5.32], [-5.17, -3.50, 3.17], [-1.36, -6.05, 1.65], [-1.84, -5.86, -0.19], [-0.15, -2.98, -1.94]]),
    // Cuello (~2.9 R) que se abre en una base en punta.
    ("neck", &[[1.55, -3.81, 1.42], [0.36, -4.94, -0.79], [0.05, -8.48, 1.59], [1.38, -8.08, -1.31], [-1.55, -3.81, 1.42], [-0.36, -4.94, -0.79], [-0.05, -8.48, 1.59], [-1.38, -8.08, -1.31]]),
    ("collar", &[[0.00, -9.41, 2.42], [0.00, -8.05, -1.84], [1.28, -8.25, 1.60], [1.06, -6.82, -1.21], [2.22, -8.61, 1.40], [-1.28, -8.25, 1.60], [-1.06, -6.82, -1.21], [-2.22, -8.61, 1.40]]),
    // Dos púas en el eje de la nuca: solo se ven de perfil.
    ("backspike", &[[0.00, 3.21, -8.01], [0.00, 0.11, -1.28], [0.23, -0.57, -0.43], [0.29, -2.43, -2.78], [-0.23, -0.57, -0.43], [-0.29, -2.43, -2.78]]),
    ("backlow", &[[0.00, -1.44, -5.61], [0.00, 0.11, 2.39], [1.36, -1.09, -2.35], [1.76, -3.28, -1.11], [-1.36, -1.09, -2.35], [-1.76, -3.28, -1.11]]),
    // Picos de la corona (sin la púa central del refsheet: se quitó a propósito).
    ("crown_R", &[[4.64, 3.08, 0.54], [2.68, 2.36, 1.85], [4.88, 1.20, 2.83], [2.07, 0.07, 0.55], [2.17, 1.27, -1.58]]),
    ("crown_L", &[[-4.64, 3.08, 0.54], [-2.68, 2.36, 1.85], [-4.88, 1.20, 2.83], [-2.07, 0.07, 0.55], [-2.17, 1.27, -1.58]]),
    // Cuernos: salen de una muesca en V profunda y suben a ~67° hacia fuera y atrás.
    ("horn_R", &[[7.90, 4.83, -1.80], [5.58, -1.41, -1.16], [5.25, -2.25, -0.48], [4.62, 0.38, -2.54], [6.81, 0.91, -2.47]]),
    ("horn_L", &[[-7.90, 4.83, -1.80], [-5.58, -1.41, -1.16], [-5.25, -2.25, -0.48], [-4.62, 0.38, -2.54], [-6.81, 0.91, -2.47]]),
    // Púas bajas casi horizontales, a la altura del borde inferior de la placa: lo más ancho.
    ("lowspike_R", &[[7.97, -1.36, 1.09], [6.98, -1.35, 2.20], [4.97, -3.80, 1.83], [2.32, -2.86, -1.45], [6.66, -1.47, 1.41]]),
    ("lowspike_L", &[[-7.97, -1.36, 1.09], [-6.98, -1.35, 2.20], [-4.97, -3.80, 1.83], [-2.32, -2.86, -1.45], [-6.66, -1.47, 1.41]]),
];

/// Giro actual + su cara proyectada.
pub struct Head {
    angles: Option<[f32; 3]>,
    data: HeadData,
}

impl Default for Head {
    fn default() -> Self {
        Self { angles: None, data: HeadData::zeroed() }
    }
}

impl Head {
    /// Ángulos actuales en radianes. Devuelve la cara nueva solo si cambió el giro.
    pub fn update(&mut self, yaw: f32, pitch: f32, roll: f32) -> Option<&HeadData> {
        let next = [yaw, pitch, roll];
        if self.angles == Some(next) {
            return None;
        }
        self.angles = Some(next);
        self.data = build(&Turn::new(yaw, pitch, roll));
        Some(&self.data)
    }
}

fn cross(o: V2, a: V2, b: V2) -> f32 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

/// Casco convexo 2D (cadena monótona), en sentido antihorario y sin puntos alineados.
fn hull(mut pts: Vec<V2>) -> Vec<V2> {
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let mut h: Vec<V2> = Vec::with_capacity(2 * pts.len());
    for pass in 0..2 {
        let start = h.len();
        for &p in &pts {
            while h.len() >= start + 2 && cross(h[h.len() - 2], h[h.len() - 1], p) <= 1e-6 {
                h.pop();
            }
            h.push(p);
        }
        h.pop(); // el último es el primero de la otra mitad
        if pass == 0 {
            pts.reverse();
        }
    }
    h
}

/// Polígonos convexos de la cara en pantalla con este giro.
fn polygons(turn: &Turn) -> Vec<Vec<V2>> {
    PARTS.iter().map(|(_, pts)| hull(pts.iter().map(|&p| turn.project(p)).collect())).collect()
}

fn build(turn: &Turn) -> HeadData {
    let mut data = HeadData::zeroed();
    let (mut n, mut first, mut reach) = (0, 0, 0.0f32);
    for poly in polygons(turn) {
        // de canto una pieza puede quedar en un segmento: no tapa nada
        if poly.len() < 3 {
            continue;
        }
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for (i, v) in poly.iter().enumerate() {
            let k = 2 * ((first + i) % 2);
            data.verts[(first + i) / 2][k..k + 2].copy_from_slice(v);
            lo = [lo[0].min(v[0]), lo[1].min(v[1])];
            hi = [hi[0].max(v[0]), hi[1].max(v[1])];
            reach = reach.max(v[0].hypot(v[1]));
        }
        data.parts[2 * n] = [first as f32, poly.len() as f32, 0.0, 0.0];
        data.parts[2 * n + 1] = [lo[0], lo[1], hi[0], hi[1]];
        first += poly.len();
        n += 1;
    }
    data.info = [n as f32, reach + 1.0, 0.0, 0.0];
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hair::HEAD_R;

    /// Dentro de un polígono convexo antihorario, con tolerancia para los puntos sobre el borde.
    fn inside(p: V2, poly: &[V2]) -> bool {
        poly.len() >= 3 && (0..poly.len()).all(|i| cross(poly[i], poly[(i + 1) % poly.len()], p) >= -1e-3)
    }

    fn turns() -> impl Iterator<Item = (f32, f32, f32)> {
        (-8..=8).flat_map(|y| (-3..=3).flat_map(move |p| (-2..=2).map(move |r| (y as f32 * 10.0, p as f32 * 15.0, r as f32 * 15.0))))
    }

    fn turn((y, p, r): (f32, f32, f32)) -> Turn {
        Turn::new(y.to_radians(), p.to_radians(), r.to_radians())
    }

    /// Punto de la esfera de la cabeza, hundido `depth`.
    fn on_head(x: f32, y: f32, depth: f32) -> [f32; 3] {
        [x, y, (HEAD_R * HEAD_R - x * x - y * y).sqrt() - depth]
    }

    #[test]
    fn fits_the_uniform_at_any_angle() {
        // en el peor caso cada punto es un vértice del casco
        let worst: usize = PARTS.iter().map(|(_, p)| p.len()).sum();
        assert!(PARTS.len() <= MAX_PARTS && worst <= MAX_VERTS, "{} piezas, {worst} vértices", PARTS.len());
    }

    #[test]
    fn hull_is_counterclockwise_and_drops_inner_points() {
        let h = hull(vec![[0.0, 0.0], [2.0, 0.0], [1.0, 0.5], [2.0, 2.0], [0.0, 2.0], [1.0, 2.0]]);
        assert_eq!(h, vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]]);
    }

    #[test]
    fn the_plate_and_the_mouth_never_leave_the_face() {
        // Placa (con margen) y zona de la boca abierta, sobre la esfera: mientras la cara mira al
        // frente, todo cae dentro de la silueta.
        let mut pts = Vec::new();
        for i in 0..=12 {
            for j in 0..=6 {
                pts.push(on_head(-4.4 + 8.8 * i as f32 / 12.0, -2.0 + 4.0 * j as f32 / 6.0, 0.0));
            }
        }
        for i in 0..=6 {
            for j in 0..=4 {
                pts.push(on_head(-2.2 + 4.4 * i as f32 / 6.0, -4.6 + 2.6 * j as f32 / 4.0, 0.2));
            }
        }
        for a in turns().filter(|a| a.0.abs() <= 70.0) {
            let t = turn(a);
            let polys = polygons(&t);
            for &p in &pts {
                let q = t.project(p);
                assert!(polys.iter().any(|poly| inside(q, poly)), "{a:?}: se sale {p:?}");
            }
        }
    }

    #[test]
    fn from_the_front_it_is_symmetric() {
        let polys = polygons(&turn((0.0, 0.0, 0.0)));
        for i in 0..40 {
            for j in 0..40 {
                let p = [-12.0 + 0.6 * i as f32 + 0.013, -11.0 + 0.6 * j as f32 + 0.017];
                let a = polys.iter().any(|poly| inside(p, poly));
                let b = polys.iter().any(|poly| inside([-p[0], p[1]], poly));
                assert_eq!(a, b, "asimetría en {p:?}");
            }
        }
    }

    #[test]
    fn same_angles_same_face() {
        let mut head = Head::default();
        let a = *head.update(0.3, -0.2, 0.1).unwrap();
        assert!(head.update(0.3, -0.2, 0.1).is_none(), "sin cambios no se sube nada");
        let b = build(&Turn::new(0.3, -0.2, 0.1));
        assert_eq!(bytemuck::bytes_of(&a), bytemuck::bytes_of(&b));
    }
}
