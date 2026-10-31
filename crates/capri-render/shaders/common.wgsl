// ─── common: uniform + primitivas SDF ────────────────────────────────────────
// El orden de campos DEBE coincidir con `Frame` en src/frame.rs.

struct Frame {
  res: vec4<f32>,      // ancho, alto, tiempo, premultiplicado
  root: vec4<f32>,     // pos.x, pos.y, rot, opacidad
  root2: vec4<f32>,    // escala, yaw, respiración, parpadeo
  cores: vec4<f32>,    // full, eyes, bolt, máscara visible
  eyes: vec4<f32>,     // abierto izq, abierto der, mirada x, mirada y
  face: vec4<f32>,     // brillo máscara, boca abierta, curva boca, tamaño ojo izq
  face2: vec4<f32>,    // tamaño ojo der, voz, -, -
  limbs0: vec4<f32>,   // cuello, torso, brazo izq, brazo der
  limbs1: vec4<f32>,   // pierna izq, pierna der, opacidad silueta, -
  ext: vec4<f32>,      // púas, largo púas, fragmentos, dispersión
  col_body: vec4<f32>,
  col_rim: vec4<f32>,
  col_mask: vec4<f32>,
  col_eyes: vec4<f32>,
  col_hair: vec4<f32>,
  col_tip: vec4<f32>,
  col_shell: vec4<f32>, // silueta del ojo + disco
  col_ring: vec4<f32>,  // anillo del ojo
  col_pupil: vec4<f32>, // centro del ojo
  fx: vec4<f32>,       // glow, densidad scanlines, fuerza scanlines, glitch
  fx2: vec4<f32>,      // contorno px
};

@group(0) @binding(0) var<uniform> F: Frame;

// Rotación antihoraria.
fn rot2(a: f32) -> mat2x2<f32> {
  let c = cos(a);
  let s = sin(a);
  return mat2x2<f32>(c, s, -s, c);
}

fn hash11(x: f32) -> f32 {
  return fract(sin(x * 127.1) * 43758.5453);
}

fn hash21(p: vec2<f32>) -> f32 {
  return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

// Unión suave: fusiona formas orgánicamente.
fn smin(a: f32, b: f32, k: f32) -> f32 {
  let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
  return mix(b, a, h) - k * h * (1.0 - h);
}

// Segmento con radio que va de `ra` (en a) a `rb` (en b).
fn sd_capsule(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, ra: f32, rb: f32) -> f32 {
  let pa = p - a;
  let ba = b - a;
  let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
  return length(pa - ba * h) - mix(ra, rb, h);
}

fn sd_box(p: vec2<f32>, half_size: vec2<f32>, r: f32) -> f32 {
  let q = abs(p) - half_size + vec2<f32>(r);
  return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

// Elipse aproximada (suficiente para ojos y torso).
fn sd_ellipse(p: vec2<f32>, r: vec2<f32>) -> f32 {
  let k0 = length(p / r);
  let k1 = max(length(p / (r * r)), 1e-5);
  return k0 * (k0 - 1.0) / k1;
}

// Triángulo exacto (Inigo Quilez). Negativo dentro.
fn sd_triangle(p: vec2<f32>, p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>) -> f32 {
  let e0 = p1 - p0;
  let e1 = p2 - p1;
  let e2 = p0 - p2;
  let v0 = p - p0;
  let v1 = p - p1;
  let v2 = p - p2;
  let q0 = v0 - e0 * clamp(dot(v0, e0) / dot(e0, e0), 0.0, 1.0);
  let q1 = v1 - e1 * clamp(dot(v1, e1) / dot(e1, e1), 0.0, 1.0);
  let q2 = v2 - e2 * clamp(dot(v2, e2) / dot(e2, e2), 0.0, 1.0);
  let s = sign(e0.x * e2.y - e0.y * e2.x);
  let d = min(min(vec2<f32>(dot(q0, q0), s * (v0.x * e0.y - v0.y * e0.x)),
                  vec2<f32>(dot(q1, q1), s * (v1.x * e1.y - v1.y * e1.x))),
                  vec2<f32>(dot(q2, q2), s * (v2.x * e2.y - v2.y * e2.x)));
  return -sqrt(d.x) * sign(d.y);
}

// Disco cuyo arco menor une `a` con `b`, abombado `sag` (flecha) hacia la izquierda de a→b.
// La intersección de dos de estos discos da una almendra con puntas agudas.
fn sd_arc_disc(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, sag: f32) -> f32 {
  let ab = b - a;
  let h = 0.5 * length(ab);
  let left = vec2<f32>(-ab.y, ab.x) / (2.0 * h);
  let r = (h * h + sag * sag) / (2.0 * sag);
  return length(p - (0.5 * (a + b) + left * (sag - r))) - r;
}

// Cobertura antialias de un SDF dado el tamaño de píxel.
fn aa(d: f32, px: f32) -> f32 {
  return clamp(0.5 - d / px, 0.0, 1.0);
}

// Compone `src` sobre `dst` (alfa no premultiplicado).
fn over(dst: vec4<f32>, src: vec4<f32>) -> vec4<f32> {
  let a = src.a + dst.a * (1.0 - src.a);
  let rgb = (src.rgb * src.a + dst.rgb * dst.a * (1.0 - src.a)) / max(a, 1e-5);
  return vec4<f32>(rgb, a);
}
