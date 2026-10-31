// ─── main: TransformController + composición de capas ─────────────────────────
//   volumen (silueta | rayo) → fragmentos → cabeza (pelo, cara, ojos) → FX

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
  let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
  return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

// Forma combinada según los pesos de los núcleos. El pelo no va aquí: es parte de la cabeza
// (hair.wgsl, dentro de la capa de cara).
fn form_sdf(p: vec2<f32>, s: Skel) -> f32 {
  let wf = F.cores.x;
  let we = F.cores.y;
  let wb = F.cores.z;
  let wsum = max(wf + we + wb, 1e-3);
  let body = sd_silhouette(p, s);
  let bolt = sd_bolt(p);
  // Mezclar campos de distancia = morph orgánico. El núcleo `eyes` encoge el volumen a nada.
  return (body * wf + bolt * wb) / max(wf + wb, 1e-3) + (we / wsum) * 0.3;
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
  let res = F.res.xy;
  let x = frag.x + glitch_offset(frag.y / res.y) * res.x;

  // TransformController: posición, rotación, escala, yaw
  var p = (vec2<f32>(x, res.y - frag.y) - 0.5 * res) / res.y;
  let scale = max(F.root2.x, 0.05);
  p = p - vec2<f32>(F.root.x * 0.5 * res.x / res.y, F.root.y * 0.5);
  p = rot2(-F.root.z) * p / scale;
  let yaw = F.root2.y;
  let squash = max(cos(yaw), 0.45);
  let pb = vec2<f32>(p.x / squash, p.y); // espacio del cuerpo (girado)
  let px = 1.0 / (res.y * scale);

  let s = skeleton();
  let wf = F.cores.x;
  let we = F.cores.y;
  let wb = F.cores.z;
  let wsum = max(wf + we + wb, 1e-3);
  let bolt_w = wb / max(wf + wb, 1e-3);
  let body_alpha = F.limbs1.z;

  // ── Capa de volumen: SDF con volumen falso (normal del gradiente + luz que sigue al yaw)
  let d = form_sdf(pb, s);
  let e = px * 1.5;
  let n = normalize(vec2<f32>(form_sdf(pb + vec2<f32>(e, 0.0), s) - d, form_sdf(pb + vec2<f32>(0.0, e), s) - d) + 1e-6);
  let light = normalize(vec2<f32>(sin(yaw * 1.5 + 0.7), 0.6));
  let lit = clamp(dot(n, light), 0.0, 1.0);
  let inner = clamp(-d / 0.035, 0.0, 1.0);
  let rim = 1.0 - smoothstep(0.0, F.fx2.x * px * 2.5, -d);

  var bcol = F.col_body.rgb * (0.7 + 0.6 * inner * lit);
  bcol = mix(bcol, F.col_rim.rgb, rim * (0.35 + 0.65 * lit));
  // Rayo: núcleo blanco, borde del color de los ojos
  bcol = mix(bcol, mix(F.col_eyes.rgb, vec3<f32>(1.0), inner), bolt_w);

  let dfr = sd_fragments(pb, vec2<f32>(0.0, -0.05 * bolt_w), 0.28);
  let energy = mix(F.col_rim.rgb, F.col_eyes.rgb, bolt_w);
  var col = vec4<f32>(0.0);
  let halo = exp(-max(min(d, dfr), 0.0) * 55.0) * F.fx.x * (0.35 + 0.4 * bolt_w) * body_alpha;
  col = over(col, vec4<f32>(energy, halo));
  col = over(col, vec4<f32>(bcol, aa(d, px) * body_alpha));
  col = over(col, vec4<f32>(energy * 1.2, aa(dfr, px) * body_alpha));

  // ── Capa de cara: sigue a la cabeza en `full`, centrada en `eyes`, arriba del rayo en `bolt`
  let c_full = vec2<f32>(s.head.x * squash, s.head.y);
  let center = (c_full * wf + vec2<f32>(0.0, 0.02) * we + vec2<f32>(0.0, 0.27) * wb) / wsum;
  let fscale = (1.0 * wf + 2.2 * we + 0.9 * wb) / wsum;
  col = over(col, face_layer(p, center, s.head_rot * wf / wsum, fscale, wf / wsum, px));

  // ── FX
  var rgb = col.rgb * scanlines(frag.y / res.y);
  let a = col.a * F.root.w;
  if (F.res.w > 0.5) {
    rgb = rgb * a;
  }
  return vec4<f32>(rgb, a);
}
