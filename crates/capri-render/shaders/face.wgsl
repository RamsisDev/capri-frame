// ─── Face: EyeLeft, EyeRight + su placa (Mask) ────────────────────────────────
// Capa 2D nítida pegada a la cabeza. Existe en todos los núcleos.
// La placa es parte de los ojos: siempre se dibuja con ellos. La boca aún no se dibuja.
//
// Las proporciones van en unidades de R = radio exterior del anillo del ojo.
// Espacio de ojo: origen en el centro del ojo en reposo, y hacia arriba y x positiva HACIA EL
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
const LID_HALF: f32 = 0.055;                              // media altura de la raya horizontal cerrada
const LID_MIN_PX: f32 = 1.1;                              // grosor mínimo del párpado cerrado

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

// Efectos propios de la cara (halo, brillo). Escalan con `style.glow`: 0 = colores planos.
const FX_FULL: f32 = 0.6;                                 // style.glow con el que llegan a su fuerza completa

// Centro y radio del disco que define cada arco de la almendra abierta.
fn eye_arc_circle(a: vec2<f32>, b: vec2<f32>, sag: f32) -> vec3<f32> {
  let ab = b - a;
  let h = 0.5 * length(ab);
  let left = vec2<f32>(-ab.y, ab.x) / (2.0 * h);
  let r = (h * h + sag * sag) / (2.0 * sag);
  let center = 0.5 * (a + b) + left * (sag - r);
  return vec3<f32>(center, r);
}

// Cada arco se convierte en una elipse al cerrar, con la misma compresión vertical
// que sus esquinas y púas. Así las púas siguen unidas durante toda la animación.
fn sd_eye_arc(e: vec2<f32>, a: vec2<f32>, b: vec2<f32>, sag: f32, opening: f32) -> f32 {
  let circle = eye_arc_circle(a, b, sag);
  let squash = vec2<f32>(1.0, opening);
  return sd_ellipse(e - circle.xy * squash, vec2<f32>(circle.z) * squash);
}

fn sd_eye_lens(e: vec2<f32>, opening: f32) -> f32 {
  return max(sd_eye_arc(e, SHELL_INNER, SHELL_OUTER, SHELL_SAG_TOP, opening),
             sd_eye_arc(e, SHELL_OUTER, SHELL_INNER, SHELL_SAG_BOTTOM, opening));
}

fn sd_eye_shell(e: vec2<f32>, opening: f32) -> f32 {
  let squash = vec2<f32>(1.0, opening);
  let thorns = min(min(sd_triangle(e, THORN_TOP_TIP * squash, THORN_TOP_A * squash, THORN_TOP_B * squash),
                       sd_triangle(e, THORN_LOW_TIP * squash, THORN_LOW_A * squash, THORN_LOW_B * squash)),
                   sd_triangle(e, HEEL_TIP * squash, HEEL_A * squash, HEEL_B * squash));
  return min(sd_eye_lens(e, opening), thorns);
}

// Recorrido hasta que el círculo central toca un arco. Restar su radio al disco
// del arco limita el círculo entero, no solo su centro. Todo en unidades de silueta.
fn eye_arc_travel(direction: vec2<f32>, pupil_radius: f32, circle: vec3<f32>) -> f32 {
  let radius = max(circle.z - pupil_radius, 0.0);
  let projection = dot(direction, circle.xy);
  let discriminant = projection * projection + radius * radius - dot(circle.xy, circle.xy);
  return max(projection + sqrt(max(discriminant, 0.0)), 0.0);
}

fn eye_gaze_offset(gaze: vec2<f32>, pupil_radius: f32) -> vec2<f32> {
  let amount = length(gaze);
  if (amount < 1e-5) {
    return vec2<f32>(0.0);
  }
  let direction = gaze / amount;
  let top = eye_arc_circle(SHELL_INNER, SHELL_OUTER, SHELL_SAG_TOP);
  let bottom = eye_arc_circle(SHELL_OUTER, SHELL_INNER, SHELL_SAG_BOTTOM);
  let travel = min(eye_arc_travel(direction, pupil_radius, top),
                   eye_arc_travel(direction, pupil_radius, bottom));
  return direction * min(amount, 1.0) * travel;
}

// Un ojo. `p` y `px` en unidades R, espacio de ojo sin escalar; `gaze` ya espejado y con
// longitud <= 1; `open` ya incluye el parpadeo. Se cierra la almendra completa hacia una
// raya horizontal; los círculos conservan su forma detrás de ella y solo se recortan.
fn eye_layer(p: vec2<f32>, size: f32, gaze: vec2<f32>, open: f32, px: f32) -> vec4<f32> {
  var col = vec4<f32>(0.0);
  let shell_k = 1.0 + SHELL_SIZE_FOLLOW * (size - 1.0);
  let s = p / shell_k;
  // La mirada solo la limita el círculo central contra la almendra abierta, con
  // margen de antialias. El parpadeo tapa la mirada sin desplazarla ni deformarla.
  let offset = eye_gaze_offset(gaze, (PUPIL_R * size + px) / shell_k) * shell_k;
  let e = (p - offset) / size;            // anillo, azul y centro comparten posición
  let r = length(e);

  // El contorno es el ojo: toda la silueta converge a un segmento sin curvatura.
  // El último 2 % mezcla con el segmento exacto para evitar elipses y triángulos
  // degenerados al llegar a cero.
  let opening = max(open, 0.02);
  let closing = min(open / 0.02, 1.0);
  let d_line = sd_capsule(s, vec2<f32>(SHELL_OUTER.x, 0.0), vec2<f32>(HEEL_TIP.x, 0.0), 0.0, 0.0);
  let closed_half = max(LID_HALF * shell_k, 0.5 * LID_MIN_PX * px);
  let d_shell = mix(d_line, sd_eye_shell(s, opening), closing) * shell_k - (1.0 - open) * closed_half;
  col = over(col, vec4<f32>(F.col_shell.rgb, aa(d_shell, px)));

  // Apertura dentro de la almendra, sin las púas. Un margen de medio píxel deja el
  // contorno delante y garantiza que con open = 0 no asome ningún círculo.
  let d_lid = mix(d_line, sd_eye_lens(s, opening), closing) * shell_k + 0.5 * px;
  let pxe = px / size;

  // El anillo pertenece al iris: se desplaza con él y puede ocultarse tras el contorno.
  let ring_half = max(0.5 * (1.0 - RING_IN), 0.5 * RING_MIN_PX * pxe);
  let d_ring = (abs(r - 1.0 + ring_half) - ring_half) * size;
  col = over(col, vec4<f32>(F.col_ring.rgb, aa(max(d_ring, d_lid), px)));

  // Azul y centro permanecen concéntricos con el anillo, sin aplastarse al cerrar.
  col = over(col, vec4<f32>(F.col_eyes.rgb, aa(max((r - IRIS_R) * size, d_lid), px)));
  col = over(col, vec4<f32>(F.col_pupil.rgb, aa(max((r - PUPIL_R) * size, d_lid), px)));
  return col;
}

fn face_layer(p: vec2<f32>, center: vec2<f32>, ang: f32, fscale: f32, px: f32) -> vec4<f32> {
  var q = rot2(-ang) * (p - center) / fscale;
  let yaw = F.root2.y;
  let turn = max(cos(yaw), 0.4);
  q.x = (q.x - sin(yaw) * 0.025) / turn; // los rasgos se desplazan hacia donde gira y se estrechan
  let u = q / EYE_R;                     // espacio de placa, unidades R
  let pxu = px / (fscale * EYE_R);
  let voice = F.face2.y;
  let glow = F.face.x + voice * 0.5;
  let fxk = clamp(F.fx.x / FX_FULL, 0.0, 1.0);
  var col = vec4<f32>(0.0);

  // Mask + halo
  let dm = sd_box(u, PLATE_HALF, PLATE_CORNER);
  let plate = aa(dm, pxu);
  let halo = exp(-max(dm, 0.0) * 1.4) * glow * 0.45 * fxk;
  col = over(col, vec4<f32>(F.col_mask.rgb, halo));
  col = over(col, vec4<f32>(F.col_mask.rgb * mix(1.0, 0.75 + 0.35 * glow, fxk), plate));

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
    let eye = eye_layer(e, size, gaze * mirror, open, pxu);
    col = over(col, vec4<f32>(eye.rgb, eye.a * plate)); // recortado a la placa (ojos ampliados incluidos)
  }
  return col;
}
