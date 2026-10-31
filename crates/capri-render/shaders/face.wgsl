// ─── Face: Mask, EyeLeft, EyeRight, Mouth ────────────────────────────────────
// Capa 2D nítida pegada a la cabeza. Existe en todos los núcleos.
//
// Las proporciones van en unidades de R = radio exterior del anillo del ojo.
// Espacio de ojo: origen en el centro del anillo, y hacia arriba y x positiva HACIA EL
// CENTRO de la cara; así una sola función dibuja los dos ojos (el derecho es el espejo).

const EYE_R: f32 = 0.0195;                                // R en unidades de cara

// Placa (relativa a su centro)
const PLATE_HALF = vec2<f32>(4.04, 1.654);                // 2.44 : 1
const PLATE_CORNER: f32 = 0.05;
const EYE_X: f32 = 2.03;                                  // separación entre centros = 4.06 R
const EYE_Y: f32 = -0.365;                                // los ojos van algo bajo el centro

// Ojo, de fuera hacia dentro
const RING_IN: f32 = 0.905;                               // anillo: 0.905 R .. 1 R
const RING_MIN_PX: f32 = 1.1;                             // grosor mínimo del anillo en píxeles
const IRIS_R: f32 = 0.585;
const PUPIL_R: f32 = 0.305;
const SHELL_SIZE_FOLLOW: f32 = 0.5;                       // la silueta crece la mitad que el disco con `size`
const GAZE_MAX: f32 = 0.27;                               // 0.585 + 0.27 < 0.905: el iris no toca el anillo

// Silueta: almendra (dos arcos entre dos esquinas en punta) + tres triángulos: las dos púas
// y el talón que afila la esquina interior. Ajustada contra la referencia (~2 % de error de área).
const SHELL_OUTER = vec2<f32>(-1.702, 1.266);             // esquina exterior, arriba
const SHELL_INNER = vec2<f32>(1.436, -0.693);             // esquina interior, abajo
const SHELL_SAG_TOP: f32 = 1.192;                         // flecha del arco superior
const SHELL_SAG_BOTTOM: f32 = 0.935;                      // flecha del arco inferior
const THORN_TOP_TIP = vec2<f32>(1.02, 1.78);              // púa superior: apunta al centro de la cara
const THORN_TOP_A = vec2<f32>(0.272, 1.096);
const THORN_TOP_B = vec2<f32>(0.564, 0.905);
const THORN_LOW_TIP = vec2<f32>(-1.258, -1.2);            // púa inferior: apunta hacia fuera
const THORN_LOW_A = vec2<f32>(-0.892, -0.561);
const THORN_LOW_B = vec2<f32>(-0.639, -0.665);
const HEEL_TIP = vec2<f32>(1.476, -0.987);                // talón: esquina inferior hacia el centro
const HEEL_A = vec2<f32>(0.497, -0.806);
const HEEL_B = vec2<f32>(1.29, -0.022);

// Boca: en el hueco entre las dos siluetas, por encima de sus talones.
const MOUTH_Y: f32 = -1.04;
const MOUTH_HALF_W: f32 = 0.42;
const MOUTH_THICK: f32 = 0.07;                            // media altura cerrada
const MOUTH_OPEN: f32 = 0.34;                             // media altura extra con boca abierta
const MOUTH_CURVE: f32 = 0.22;

// Efectos propios de la cara (halo, brillo). Escalan con `style.glow`: 0 = colores planos.
const FX_FULL: f32 = 0.6;                                 // style.glow con el que llegan a su fuerza completa

fn sd_eye_shell(e: vec2<f32>) -> f32 {
  let lens = max(sd_arc_disc(e, SHELL_INNER, SHELL_OUTER, SHELL_SAG_TOP),
                 sd_arc_disc(e, SHELL_OUTER, SHELL_INNER, SHELL_SAG_BOTTOM));
  let thorns = min(min(sd_triangle(e, THORN_TOP_TIP, THORN_TOP_A, THORN_TOP_B),
                       sd_triangle(e, THORN_LOW_TIP, THORN_LOW_A, THORN_LOW_B)),
                   sd_triangle(e, HEEL_TIP, HEEL_A, HEEL_B));
  return min(lens, thorns);
}

// Un ojo. `p` y `px` en unidades R, espacio de ojo sin escalar; `gaze` ya espejado y con
// longitud <= 1; `open` ya incluye el parpadeo. Silueta y anillo quedan fijos; iris y centro
// se mueven. Las distancias se devuelven a unidades sin escalar para que el antialias no cambie.
fn eye_layer(p: vec2<f32>, size: f32, gaze: vec2<f32>, open: f32, px: f32, lone: f32, fxk: f32) -> vec4<f32> {
  var col = vec4<f32>(0.0);
  let shell_k = 1.0 + SHELL_SIZE_FOLLOW * (size - 1.0);
  let e = p / size;                      // disco, anillo, iris y centro
  let r = length(e);

  // Silueta + disco negro (sin placa y con efectos, un halo del color del anillo)
  let d_shell = min(sd_eye_shell(p / shell_k) * shell_k, (r - 1.0) * size);
  col = over(col, vec4<f32>(F.col_ring.rgb, exp(-max(d_shell, 0.0) * 1.6) * 0.5 * lone * fxk));
  col = over(col, vec4<f32>(F.col_shell.rgb, aa(d_shell, px)));
  let pxe = px / size;

  // Anillo: el borde exterior queda en 1 R aunque se engrose para ser legible
  let ring_half = max(0.5 * (1.0 - RING_IN), 0.5 * RING_MIN_PX * pxe);
  let inner = 1.0 - 2.0 * ring_half;
  col = over(col, vec4<f32>(F.col_ring.rgb, aa(abs(r - 1.0 + ring_half) - ring_half, pxe)));

  // Párpados: ventana elíptica dentro del disco que se cierra hacia la línea media.
  // Con open = 0 la ventana queda un píxel por debajo de cero: no asoma nada.
  let lid_h = open * inner - pxe;
  let ex = e.x / inner;
  let d_lid = max(abs(e.y) - lid_h * sqrt(max(1.0 - ex * ex, 0.0)), r - inner);

  // Iris y centro, recortados por los párpados
  let g = length(e - gaze * GAZE_MAX);
  col = over(col, vec4<f32>(F.col_eyes.rgb * (1.0 + 0.5 * lone * fxk), aa(max(g - IRIS_R, d_lid), pxe)));
  col = over(col, vec4<f32>(F.col_pupil.rgb, aa(max(g - PUPIL_R, d_lid), pxe)));
  return col;
}

fn face_layer(p: vec2<f32>, center: vec2<f32>, ang: f32, fscale: f32, px: f32) -> vec4<f32> {
  var q = rot2(-ang) * (p - center) / fscale;
  let yaw = F.root2.y;
  let turn = max(cos(yaw), 0.4);
  q.x = (q.x - sin(yaw) * 0.025) / turn; // los rasgos se desplazan hacia donde gira y se estrechan
  let u = q / EYE_R;                     // espacio de placa, unidades R
  let pxu = px / (fscale * EYE_R);
  let mask_vis = F.cores.w;
  let lone = 1.0 - mask_vis;             // sin placa, los ojos son todo lo que queda
  let voice = F.face2.y;
  let glow = F.face.x + voice * 0.5;
  let fxk = clamp(F.fx.x / FX_FULL, 0.0, 1.0);
  var col = vec4<f32>(0.0);

  // Mask + halo
  let dm = sd_box(u, PLATE_HALF, PLATE_CORNER);
  let plate = aa(dm, pxu);
  let halo = exp(-max(dm, 0.0) * 1.4) * glow * 0.45 * fxk * mask_vis;
  col = over(col, vec4<f32>(F.col_mask.rgb, halo));
  col = over(col, vec4<f32>(F.col_mask.rgb * mix(1.0, 0.75 + 0.35 * glow, fxk), plate * mask_vis));
  // Con placa, ojos y boca quedan recortados dentro de ella (ojos ampliados incluidos).
  let clip = mix(1.0, plate, mask_vis);

  // Eyes
  let blink = F.root2.w;
  var gaze = F.eyes.zw;
  let gl = length(gaze);
  if (gl > 1.0) {
    gaze = gaze / gl;
  }
  for (var i = 0; i < 2; i++) {
    let right = i == 1;
    let side = select(-1.0, 1.0, right);
    let mirror = vec2<f32>(-side, 1.0);
    let open = clamp(select(F.eyes.x, F.eyes.y, right), 0.0, 1.0) * (1.0 - blink);
    let size = max(select(F.face.w, F.face2.x, right), 0.05);
    let e = (u - vec2<f32>(side * EYE_X, EYE_Y)) * mirror;
    let eye = eye_layer(e, size, gaze * mirror, open, pxu, lone, fxk);
    col = over(col, vec4<f32>(eye.rgb, eye.a * clip));
  }

  // Mouth (dibujada sobre la placa)
  let mx = u.x / MOUTH_HALF_W;
  if (abs(mx) < 1.0) {
    let yc = MOUTH_Y + F.face.z * MOUTH_CURVE * (mx * mx - 0.5);
    let thick = MOUTH_THICK + F.face.y * MOUTH_OPEN * (1.0 - mx * mx);
    let dmo = abs(u.y - yc) - thick;
    col = over(col, vec4<f32>(F.col_body.rgb, aa(dmo, pxu) * mask_vis * plate));
  }
  return col;
}
