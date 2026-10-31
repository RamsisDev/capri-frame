// ─── Face: Mask, EyeLeft, EyeRight, Mouth ────────────────────────────────────
// Capa 2D nítida pegada a la cabeza. Existe en todos los núcleos.

fn face_layer(p: vec2<f32>, center: vec2<f32>, ang: f32, fscale: f32, px: f32) -> vec4<f32> {
  var q = rot2(-ang) * (p - center) / fscale;
  let pxq = px / fscale;
  let yaw = F.root2.y;
  let turn = max(cos(yaw), 0.4);
  q.x = q.x - sin(yaw) * 0.025; // los rasgos se desplazan hacia donde gira
  let mask_vis = F.cores.w;
  let voice = F.face2.y;
  let glow = F.face.x + voice * 0.5;
  var col = vec4<f32>(0.0);

  // Mask + halo
  let dm = sd_box(q, vec2<f32>(0.074 * turn, 0.048), 0.012);
  let halo = exp(-max(dm, 0.0) * 70.0) * glow * 0.45 * mask_vis;
  col = over(col, vec4<f32>(F.col_mask.rgb, halo));
  col = over(col, vec4<f32>(F.col_mask.rgb * (0.75 + 0.35 * glow), aa(dm, pxq) * mask_vis));

  // Eyes
  let gaze = vec2<f32>(F.eyes.z, F.eyes.w) * vec2<f32>(0.012, 0.009);
  let spacing = 0.032 * turn;
  let blink = F.root2.w;
  for (var i = 0; i < 2; i++) {
    let right = i == 1;
    let open = select(F.eyes.x, F.eyes.y, right) * (1.0 - blink);
    let size = select(F.face.w, F.face2.x, right);
    let c = vec2<f32>(select(-1.0, 1.0, right) * spacing, 0.006) + gaze;
    let r = vec2<f32>(0.015, 0.019) * size * vec2<f32>(1.0, max(open, 0.08));
    let de = sd_ellipse(q - c, r);
    // Sin máscara los ojos brillan más (son todo lo que queda).
    let lone = 1.0 - mask_vis;
    col = over(col, vec4<f32>(F.col_eyes.rgb, exp(-max(de, 0.0) * 90.0) * 0.6 * lone * (0.6 + glow)));
    col = over(col, vec4<f32>(F.col_eyes.rgb * (1.0 + 0.5 * lone), aa(de, pxq)));
    let dh = length(q - c - vec2<f32>(0.004, 0.006) * size) - 0.0035 * size;
    col = over(col, vec4<f32>(vec3<f32>(1.0), aa(dh, pxq) * 0.9 * smoothstep(0.25, 0.5, open)));
  }

  // Mouth (dibujada sobre la máscara)
  let mx = q.x / 0.022;
  if (abs(mx) < 1.0) {
    let yc = -0.027 + F.face.z * 0.008 * (mx * mx - 0.5);
    let thick = 0.0022 + F.face.y * 0.009 * (1.0 - mx * mx);
    let dmo = abs(q.y - yc) - thick;
    col = over(col, vec4<f32>(F.col_body.rgb, aa(dmo, pxq) * mask_vis));
  }
  return col;
}
