// ─── Extensions: Fragments + forma rayo (núcleo bolt). El pelo vive en hair.wgsl ──

// Piezas sueltas en órbita. Sin fusión: se ven separadas del cuerpo.
fn sd_fragments(p: vec2<f32>, center: vec2<f32>, radius: f32) -> f32 {
  let amount = F.ext.z;
  if (amount < 0.01) {
    return 1e3;
  }
  let t = F.res.z;
  var d = 1e3;
  for (var i = 0; i < 10; i++) {
    let fi = f32(i);
    let a = fi * 0.628 + t * (0.4 + 0.3 * hash11(fi));
    let r = radius * F.ext.w * (0.6 + 0.6 * hash11(fi + 7.0));
    let c = center + vec2<f32>(cos(a) * r, sin(a) * r * 1.6);
    let q = rot2(t * 2.0 + fi) * (p - c);
    d = min(d, sd_box(q, vec2<f32>(0.012, 0.007) * amount * (0.6 + hash11(fi + 2.0)), 0.001));
  }
  return d;
}

// Rayo en zigzag que cambia de forma a saltos (energía inestable).
fn sd_bolt(p: vec2<f32>) -> f32 {
  let seed = floor(F.res.z * 9.0);
  var d = 1e3;
  var prev = vec2<f32>(0.0, 0.2);
  for (var i = 1; i <= 7; i++) {
    let fi = f32(i);
    let dir = select(-1.0, 1.0, (i % 2) == 0);
    let x = select(dir * (0.025 + 0.05 * hash21(vec2<f32>(fi, seed))), 0.0, i == 7);
    let cur = vec2<f32>(x, 0.2 - fi * 0.092);
    d = min(d, sd_capsule(p, prev, cur, 0.014, 0.011));
    prev = cur;
  }
  return d;
}
