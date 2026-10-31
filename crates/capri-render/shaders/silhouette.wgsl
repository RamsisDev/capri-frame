// ─── Silhouette: Torso, Arms, Legs ───────────────────────────────────────────
// Espacio del cuerpo: alto de ventana = 1, centro = (0,0), y hacia arriba.

struct Skel {
  hip: vec2<f32>,
  neck: vec2<f32>,
  head: vec2<f32>,
  shoulder_l: vec2<f32>,
  shoulder_r: vec2<f32>,
  up: vec2<f32>,
  side: vec2<f32>,
  head_rot: f32,
};

// Cinemática directa a partir de los ángulos del stream `body`.
fn skeleton() -> Skel {
  let breath = F.root2.z;
  let lean = F.limbs0.y;
  var s: Skel;
  s.up = rot2(lean) * vec2<f32>(0.0, 1.0);
  s.side = rot2(lean) * vec2<f32>(1.0, 0.0);
  s.hip = vec2<f32>(0.0, -0.06);
  s.neck = s.hip + s.up * (0.22 + 0.006 * breath);
  s.head_rot = lean + F.limbs0.x;
  s.head = s.neck + rot2(s.head_rot) * vec2<f32>(0.0, 0.085);
  s.shoulder_l = s.neck - s.side * 0.088 - s.up * 0.025;
  s.shoulder_r = s.neck + s.side * 0.088 - s.up * 0.025;
  return s;
}

fn sd_silhouette(p: vec2<f32>, s: Skel) -> f32 {
  let lean = F.limbs0.y;
  let to_torso = rot2(-lean);
  // Torso: columna + pecho + cadera
  var d = sd_capsule(p, s.hip, s.neck - s.up * 0.02, 0.032, 0.04);
  d = smin(d, sd_ellipse(to_torso * (p - mix(s.hip, s.neck, 0.7)), vec2<f32>(0.056, 0.044)), 0.02);
  d = smin(d, sd_ellipse(to_torso * (p - s.hip), vec2<f32>(0.05, 0.038)), 0.02);
  // Cuello + cabeza (la cabeza queda debajo de la máscara)
  d = smin(d, sd_capsule(p, s.neck - s.up * 0.02, s.head, 0.014, 0.014), 0.01);
  d = smin(d, sd_box(rot2(-s.head_rot) * (p - s.head), vec2<f32>(0.068, 0.056), 0.03), 0.015);
  // Arms: 0° = colgando (un poco abiertos); positivo = antihorario. Fusión corta: se ven separados del torso.
  let hand_l = s.shoulder_l + rot2(F.limbs0.z - 0.12) * vec2<f32>(0.0, -0.27);
  let hand_r = s.shoulder_r + rot2(F.limbs0.w + 0.12) * vec2<f32>(0.0, -0.27);
  d = smin(d, sd_capsule(p, s.shoulder_l, hand_l, 0.02, 0.011), 0.008);
  d = smin(d, sd_capsule(p, s.shoulder_r, hand_r, 0.02, 0.011), 0.008);
  // Legs
  let hip_l = s.hip - s.side * 0.03;
  let hip_r = s.hip + s.side * 0.03;
  d = smin(d, sd_capsule(p, hip_l, hip_l + rot2(F.limbs1.x) * vec2<f32>(0.0, -0.38), 0.03, 0.013), 0.02);
  d = smin(d, sd_capsule(p, hip_r, hip_r + rot2(F.limbs1.y) * vec2<f32>(0.0, -0.38), 0.03, 0.013), 0.02);
  return d;
}
