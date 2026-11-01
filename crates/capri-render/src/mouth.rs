//! Boca: un polígono blanco de aristas rectas bajo la placa, con las 8 formas de la referencia
//! de visemas (X, MBP, A, E, I, O, U, F/L).
//!
//! Todas las formas comparten topología: borde superior con `K` vértices de izquierda a derecha
//! y borde inferior con `K` de derecha a izquierda. Así cualquier mezcla de pesos es una media de
//! vértices y el paso de un visema a otro es un morph sin saltos. Los trazos finos (X y MBP) son
//! el mismo polígono con el borde inferior pegado al superior.
//!
//! - **Pesos**: `viseme.*` (stream `voice`). Lo que falta hasta 1 es la boca de reposo: X, que
//!   con `mouth.open` (más el nivel de voz) se abre hacia la A.
//! - **Emoción** (`mouth.curve`): dobla la forma con una parábola (comisuras arriba = sonrisa,
//!   abajo = tristeza) y la ensancha o estrecha un poco. Pesa entero en X, MBP y A, algo menos
//!   en el resto.
//!
//! Unidades: R (radio del anillo del ojo), origen en el centro de la raya MBP, y hacia arriba.
//! `ORIGIN` lo coloca en el espacio de la placa (centro de la cabeza).

pub type V2 = [f32; 2];

/// Vértices por borde.
pub const K: usize = 9;
/// Vértices del polígono.
pub const VERTS: usize = 2 * K;
/// Máximo que cabe en el uniform (`MOUTH_VERTS` de face.wgsl).
pub const MAX_VERTS: usize = 24;
const _: () = assert!(VERTS <= MAX_VERTS);

pub type Poly = [V2; VERTS];

/// Centro de la raya MBP en el espacio de la placa: 0.95 R bajo el borde de la placa, centrado
/// entre los ojos (como en la referencia).
pub const ORIGIN: V2 = [0.0, -2.6];
/// Escala de las plantillas. En la referencia los ojos están a 4.33 R y en el modelo a 4.06 R
/// (×0.94); con 0.9 la punta de la A (2.1 R bajo la raya) cabe en la barbilla de nuestra cara.
const SCALE: f32 = 0.9;

// Plantillas medidas sobre la hoja de visemas (IoU 0.98–0.995 en las formas rellenas; X y MBP
// con error de línea central < 0.005 R). Unidades R de la referencia, sin escalar: `SCALE` las
// lleva al modelo. Índices del borde superior: 0 y 8 esquinas, 4 centro, 2 y 6 picos laterales.
// No son simétricas a propósito (la A tiene la esquina izquierda más alta, la I los picos
// corridos, la O un pico extra a la derecha), como en la referencia.
const X: Poly = [
    [-1.849, -0.023], [-1.301, 0.167], [-0.752, 0.357], [-0.241, -0.098], [0.157, 0.075], [0.555, 0.248], [1.055, -0.184], [1.553, 0.011], [2.052, 0.206],
    [2.052, 0.097], [1.553, -0.098], [1.055, -0.293], [0.555, 0.139], [0.157, -0.034], [-0.241, -0.207], [-0.752, 0.249], [-1.301, 0.058], [-1.849, -0.132],
];
const MBP: Poly = [
    [-1.713, 0.051], [-1.285, 0.051], [-0.856, 0.051], [-0.428, 0.051], [0.000, 0.051], [0.428, 0.051], [0.856, 0.051], [1.285, 0.051], [1.713, 0.051],
    [1.713, -0.051], [1.285, -0.051], [0.856, -0.051], [0.428, -0.051], [0.000, -0.051], [-0.428, -0.051], [-0.856, -0.051], [-1.285, -0.051], [-1.713, -0.051],
];
const A: Poly = [
    [-2.331, 0.848], [-1.459, 0.518], [-0.766, 0.797], [-0.450, 0.599], [-0.134, 0.401], [0.195, 0.548], [0.523, 0.694], [1.082, 0.334], [2.189, 0.550],
    [1.754, 0.021], [1.319, -0.508], [0.884, -1.037], [0.449, -1.566], [0.014, -2.095], [-0.455, -1.506], [-0.924, -0.917], [-1.393, -0.329], [-1.862, 0.260],
];
const E: Poly = [
    [-2.733, 0.257], [-2.023, 0.105], [-1.359, 0.442], [-0.715, 0.088], [0.001, 0.468], [0.663, 0.087], [1.354, 0.425], [1.936, 0.092], [2.769, 0.284],
    [2.094, -0.290], [1.419, -0.863], [0.945, -0.863], [0.471, -0.863], [-0.003, -0.863], [-0.477, -0.863], [-0.951, -0.864], [-1.425, -0.864], [-2.079, -0.303],
];
const I: Poly = [
    [-2.439, 0.043], [-1.470, 0.174], [-0.908, 0.472], [-0.330, 0.165], [0.257, 0.480], [0.841, 0.145], [1.426, 0.426], [1.970, 0.128], [2.667, 0.051],
    [2.178, -0.045], [1.690, -0.140], [1.202, -0.235], [0.713, -0.331], [0.225, -0.426], [-0.308, -0.332], [-0.841, -0.238], [-1.373, -0.145], [-1.906, -0.051],
];
const O: Poly = [
    [-1.441, 0.097], [-1.096, 0.404], [-0.751, 0.711], [-0.518, 0.519], [-0.285, 0.328], [0.215, 0.710], [0.675, 0.312], [0.931, 0.473], [1.245, 0.107],
    [1.243, -0.488], [0.469, -1.263], [0.274, -1.263], [0.078, -1.264], [-0.117, -1.264], [-0.312, -1.265], [-0.507, -1.265], [-0.702, -1.265], [-1.440, -0.515],
];
const U: Poly = [
    [-0.706, -0.087], [-0.518, 0.077], [-0.330, 0.241], [-0.142, 0.405], [0.047, 0.570], [0.233, 0.405], [0.420, 0.241], [0.607, 0.076], [0.793, -0.088],
    [0.645, -0.231], [0.496, -0.373], [0.347, -0.516], [0.199, -0.658], [0.050, -0.801], [-0.101, -0.658], [-0.252, -0.515], [-0.403, -0.373], [-0.555, -0.230],
];
const FL: Poly = [
    [-2.634, 0.385], [-2.025, 0.206], [-1.375, 0.534], [-0.717, 0.190], [0.008, 0.567], [0.697, 0.177], [1.381, 0.497], [1.980, 0.191], [2.656, 0.373],
    [2.156, 0.010], [1.657, -0.353], [1.100, -0.353], [0.544, -0.353], [-0.012, -0.353], [-0.568, -0.353], [-1.124, -0.352], [-1.680, -0.352], [-2.157, 0.016],
];

/// Formas en el orden de `capri_core::body::VISEMES`.
const SHAPES: [Poly; 8] = [scaled(X), scaled(MBP), scaled(A), scaled(E), scaled(I), scaled(O), scaled(U), scaled(FL)];

const fn scaled(p: Poly) -> Poly {
    let mut out = p;
    let mut i = 0;
    while i < VERTS {
        out[i] = [p[i][0] * SCALE, p[i][1] * SCALE];
        i += 1;
    }
    out
}
/// Cuánto le afecta la emoción a cada visema (X, MBP y A enteros).
const EMOTION: [f32; 8] = [1.0, 1.0, 1.0, 0.8, 0.8, 0.5, 0.4, 0.8];
/// Diferencia de altura entre comisuras y centro con `curve` = ±1, en R.
const BEND: f32 = 0.5;
/// En la sonrisa, parte de la curva que baja el centro (el resto sube las comisuras): las
/// esquinas apenas suben y nunca tocan la placa. En la tristeza bajan las comisuras y el
/// centro se queda.
const BEND_CENTER: f32 = 0.8;
/// Ensanche relativo con `curve` = 1 (sonrisa más ancha, tristeza más estrecha).
const WIDEN: f32 = 0.1;
/// Lo que se acorta la parte de abajo de la boca con cualquier emoción: la sonrisa abierta es
/// más ancha y menos honda, y la triste no se sale de la barbilla.
const DEPTH: f32 = 0.25;

/// Forma de un visema con la emoción aplicada (`k` = cuánto le afecta). La parábola va de
/// esquina a esquina de cada forma, así una forma estrecha se dobla igual de reconocible.
fn emote(poly: &Poly, k: f32, curve: f32) -> Poly {
    let c = curve * k;
    let half = poly.iter().fold(0.0f32, |m, v| m.max(v[0].abs())).max(1e-3);
    poly.map(|[x, y]| {
        let t = x / half;
        let y = if y < 0.0 { y * (1.0 - DEPTH * c.abs()) } else { y };
        let center = if c > 0.0 { BEND_CENTER } else { 0.0 };
        [x * (1.0 + WIDEN * c), y + BEND * c * (t * t - center)]
    })
}

/// Polígono de la boca para unos pesos de visema, una apertura de reposo y una emoción.
pub fn shape(weights: [f32; 8], open: f32, curve: f32) -> Poly {
    let (open, curve) = (open.clamp(0.0, 1.0), curve.clamp(-1.0, 1.0));
    let w = weights.map(|w| w.max(0.0));
    let sum: f32 = w.iter().sum();
    let norm = sum.max(1.0);
    let rest = 1.0 - sum.min(1.0);
    let mut out = [[0.0; 2]; VERTS];
    let mut add = |i: usize, k: f32| {
        if k > 0.0 {
            for (o, v) in out.iter_mut().zip(emote(&SHAPES[i], EMOTION[i], curve)) {
                o[0] += v[0] * k;
                o[1] += v[1] * k;
            }
        }
    };
    for (i, &wi) in w.iter().enumerate() {
        add(i, wi / norm);
    }
    add(0, rest * (1.0 - open)); // reposo cerrado: X
    add(2, rest * open); //          reposo abierto: A
    out
}

/// Empaqueta el polígono para el uniform: dos vértices por vec4.
pub fn pack(poly: &Poly) -> [[f32; 4]; MAX_VERTS / 2] {
    let mut out = [[0.0; 4]; MAX_VERTS / 2];
    for (i, v) in poly.iter().enumerate() {
        out[i / 2][2 * (i % 2)] = v[0];
        out[i / 2][2 * (i % 2) + 1] = v[1];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(i: usize) -> [f32; 8] {
        let mut w = [0.0; 8];
        w[i] = 1.0;
        w
    }

    fn close(a: &Poly, b: &Poly) -> bool {
        a.iter().zip(b).all(|(p, q)| (p[0] - q[0]).abs() < 1e-5 && (p[1] - q[1]).abs() < 1e-5)
    }

    #[test]
    fn rest_is_x_and_opens_to_a() {
        assert!(close(&shape([0.0; 8], 0.0, 0.0), &SHAPES[0]));
        assert!(close(&shape([0.0; 8], 1.0, 0.0), &SHAPES[2]));
    }

    #[test]
    fn each_viseme_is_its_shape_when_neutral() {
        for (i, s) in SHAPES.iter().enumerate() {
            assert!(close(&shape(one(i), 0.7, 0.0), s), "visema {i}");
        }
    }

    #[test]
    fn weights_over_one_are_normalized() {
        let half = shape([0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.0, 0.0);
        let (m, a) = (&SHAPES[1], &SHAPES[2]);
        let mid: Poly = std::array::from_fn(|i| [(m[i][0] + a[i][0]) / 2.0, (m[i][1] + a[i][1]) / 2.0]);
        assert!(close(&half, &mid));
    }

    /// Altura media de las comisuras (esquinas del borde superior) menos la del centro del borde.
    fn corner_lift(p: &Poly) -> f32 {
        (p[0][1] + p[K - 1][1]) / 2.0 - p[K / 2][1]
    }

    #[test]
    fn emotion_bends_the_first_three() {
        for i in 0..3 {
            let neutral = corner_lift(&shape(one(i), 0.0, 0.0));
            assert!(corner_lift(&shape(one(i), 0.0, 1.0)) > neutral + 0.3, "visema {i} feliz");
            assert!(corner_lift(&shape(one(i), 0.0, -1.0)) < neutral - 0.3, "visema {i} triste");
        }
    }

    /// Ninguna forma, con ninguna emoción, toca la placa (borde inferior en y = -1.654 R) ni se
    /// sale de la cara oscura (mandíbula: elipse de 4.45 × 4.15 R con centro en y = -0.4 R).
    #[test]
    fn fits_between_plate_and_chin() {
        for i in 0..8 {
            for curve in [-1.0, -0.5, 0.0, 0.5, 1.0] {
                for v in shape(one(i), 0.0, curve) {
                    let (x, y) = (v[0] + ORIGIN[0], v[1] + ORIGIN[1]);
                    let jaw = (x / 4.45).powi(2) + ((y + 0.4) / 4.15).powi(2);
                    assert!(y < -1.72 && jaw < 0.97, "visema {i} curva {curve}: ({x}, {y}) jaw {jaw}");
                }
            }
        }
    }

    #[test]
    fn packs_two_vertices_per_vec4() {
        let a = &SHAPES[2];
        let p = pack(a);
        assert_eq!(p[0], [a[0][0], a[0][1], a[1][0], a[1][1]]);
        assert_eq!(p[VERTS / 2 - 1][2..], [a[VERTS - 1][0], a[VERTS - 1][1]]);
    }
}
