// ─── Extensions: Spikes, Fragments + forma rayo (núcleo bolt) ─────────────────

// Púas en abanico sobre la cabeza. Devuelve (distancia, 0..1 posición a lo largo: 1 = punta).
fn sd_spikes(p: vec2<f32>, s: Skel) -> vec2<f32> {
  let amount = F.ext.x;
  if (amount < 0.01) {
    return vec2<f32>(1e3, 0.0);
  }
  let t = F.res.z;
  let q = rot2(-s.head_rot) * (p - s.head);
  var best = vec2<f32>(1e3, 0.0);
  for (var i = 0; i < 7; i++) {
    let fi = f32(i);
    let k = fi / 6.0 - 0.5;
    let ang = k * 2.4 + sin(t * 1.7 + fi * 1.3) * 0.06;
    let len = (0.11 + 0.06 * hash11(fi + 3.0) + 0.05 * (1.0 - abs(k) * 2.0)) * F.ext.y * amount;
    let base = vec2<f32>(k * 0.09, 0.03);
    let tip = base + vec2<f32>(sin(ang), cos(ang)) * len;
    let pa = q - base;
    let ba = tip - base;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    let d = length(pa - ba * h) - mix(0.028, 0.002, h) * amount;
    if (d < best.x) {
      best = vec2<f32>(d, h);
    }
  }
  return best;
}

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
