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

// Cierre con curva (`eye.lid`): 1 = feliz, arco ∩; -1 = triste, la curvatura opuesta (U).
// El arco es una parábola, más alta hacia la esquina exterior (como la almendra inclinada); el
// contorno cerrado es una medialuna en punta y las dos púas quedan alineadas cruzándola en X.
const LID_PEAK_X: f32 = -0.35;                            // x del punto más alto (y más grueso)
const LID_PEAK: f32 = 0.6;                                // altura del arco en ese punto
const LID_K: f32 = 0.3;                                   // apertura de la parábola
const LID_CRESCENT: f32 = 0.14;                           // media altura máxima de la medialuna
const LID_CROSS_X: f32 = 0.0;                             // donde la raya de las púas cruza el arco
const LID_SPIKE_DIR = vec2<f32>(0.617, 0.787);            // raya de las púas: hacia el centro y arriba
const LID_SPIKE_TOP: f32 = 1.17;                          // largo de la púa superior desde el cruce
const LID_SPIKE_LOW: f32 = 0.9;                           // largo de la púa inferior desde el cruce
const LID_SPIKE_HALF: f32 = 0.11;                         // media anchura de las púas en el cruce

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

// Volumen: la placa va pegada a una esfera (la cabeza) que gira con `body.yaw`, `body.pitch` y
// `body.roll`. Proyección
// ortográfica sobre el plano de la cara, así a 0° se ve exactamente el 2D. Toda la placa cabe en
// el hemisferio delantero (esquina a 4.37 R < HEAD_R), sin cortes dentro de ella.
const HEAD_R: f32 = 5.4;                                  // radio de la esfera, unidades R

// Pantalla → espacio de la cabeza. La cabeza gira yaw (eje vertical, + = la cara va a la derecha),
// luego pitch (eje lateral, + = mira arriba) y luego roll (eje de la cara, + = antihorario);
// aquí se deshacen en orden inverso. Matrices por columnas.
fn head_inverse(yaw: f32, pitch: f32, roll: f32) -> mat3x3<f32> {
  let cy = cos(yaw);
  let sy = sin(yaw);
  let cp = cos(pitch);
  let sp = sin(pitch);
  let cr = cos(roll);
  let sr = sin(roll);
  let un_yaw = mat3x3<f32>(vec3<f32>(cy, 0.0, sy), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(-sy, 0.0, cy));
  let un_pitch = mat3x3<f32>(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, cp, sp), vec3<f32>(0.0, -sp, cp));
  let un_roll = mat3x3<f32>(vec3<f32>(cr, -sr, 0.0), vec3<f32>(sr, cr, 0.0), vec3<f32>(0.0, 0.0, 1.0));
  return un_roll * un_pitch * un_yaw;
}

// Al girar, la placa se comprime en pantalla (más cerca del borde de la esfera): el antialias
// crece en la misma medida. Es el estiramiento de pantalla → placa en el punto de profundidad `z`;
// de frente vale 1.
fn head_stretch(to_head: mat3x3<f32>, w: vec2<f32>, z: f32) -> f32 {
  let k = -w / select(min(z, -1e-3), max(z, 1e-3), z >= 0.0);
  let dx = (to_head * vec3<f32>(1.0, 0.0, k.x)).xy;
  let dy = (to_head * vec3<f32>(0.0, 1.0, k.y)).xy;
  return clamp(max(length(dx), length(dy)), 1.0, 8.0);
}

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

fn sd_eye_heel(e: vec2<f32>, opening: f32) -> f32 {
  let squash = vec2<f32>(1.0, opening);
  return sd_triangle(e, HEEL_TIP * squash, HEEL_A * squash, HEEL_B * squash);
}

// Altura del arco cerrado (curva +1) y su pendiente.
fn lid_arc(x: f32) -> f32 {
  return LID_PEAK - LID_K * (x - LID_PEAK_X) * (x - LID_PEAK_X);
}

// Grosor relativo de la medialuna: 1 en el punto más alto, 0 en las dos esquinas.
fn lid_taper(x: f32) -> f32 {
  let reach = select(LID_PEAK_X - SHELL_OUTER.x, HEEL_TIP.x - LID_PEAK_X, x > LID_PEAK_X);
  let t = (x - LID_PEAK_X) / reach;
  return max(1.0 - t * t, 0.0);
}

// Vértice de una púa. Sin curva se aplasta con la almendra (como siempre); con curva viaja a
// su sitio en la raya que cruza el arco cerrado.
fn lid_vertex(v: vec2<f32>, closed: vec2<f32>, open: f32, opening: f32, lid: f32) -> vec2<f32> {
  let neutral = vec2<f32>(v.x, v.y * opening + (1.0 - open) * lid * lid_arc(v.x));
  return mix(neutral, mix(closed, v, open), abs(lid));
}

fn sd_eye_thorns(u: vec2<f32>, open: f32, opening: f32, lid: f32) -> f32 {
  let meet = vec2<f32>(LID_CROSS_X, lid * lid_arc(LID_CROSS_X));
  let side = vec2<f32>(-LID_SPIKE_DIR.y, LID_SPIKE_DIR.x) * LID_SPIKE_HALF;
  let top = sd_triangle(u, lid_vertex(THORN_TOP_TIP, meet + LID_SPIKE_DIR * LID_SPIKE_TOP, open, opening, lid),
                        lid_vertex(THORN_TOP_A, meet + side, open, opening, lid),
                        lid_vertex(THORN_TOP_B, meet - side, open, opening, lid));
  let low = sd_triangle(u, lid_vertex(THORN_LOW_TIP, meet - LID_SPIKE_DIR * LID_SPIKE_LOW, open, opening, lid),
                        lid_vertex(THORN_LOW_A, meet + side, open, opening, lid),
                        lid_vertex(THORN_LOW_B, meet - side, open, opening, lid));
  return min(top, low);
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
// `lid` curva la silueta al cerrarse (1 = feliz ∩, -1 = triste U): el espacio de la silueta se
// dobla con una parábola que crece con el cierre, así púas y almendra se doblan juntas.
fn eye_layer(p: vec2<f32>, size: f32, gaze: vec2<f32>, open: f32, lid: f32, px: f32) -> vec4<f32> {
  var col = vec4<f32>(0.0);
  let shell_k = 1.0 + SHELL_SIZE_FOLLOW * (size - 1.0);
  let u = p / shell_k;                    // espacio de silueta
  // Con curva, la almendra se dobla sobre el arco a medida que cierra: `s` es el espacio
  // doblado. El doblez estira el campo de distancia; `bend_k` lo compensa.
  let lid_c = clamp(lid, -1.0, 1.0);
  let bend = lid_c * (1.0 - open);
  let s = vec2<f32>(u.x, u.y - bend * lid_arc(u.x));
  let slope = -2.0 * LID_K * bend * (u.x - LID_PEAK_X);
  let bend_k = inverseSqrt(1.0 + slope * slope);
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
  let line_half = max(LID_HALF * shell_k, 0.5 * LID_MIN_PX * px);
  let closed_half = mix(line_half, LID_CRESCENT * shell_k * lid_taper(s.x), abs(lid_c));
  let d_body = mix(d_line, min(sd_eye_lens(s, opening), sd_eye_heel(s, opening)), closing) * shell_k * bend_k
    - (1.0 - open) * closed_half;
  // Sin curva las púas se funden con la raya en el último 2 % (como la almendra); con curva
  // siguen enteras cruzando el arco.
  let d_spikes = sd_eye_thorns(u, open, opening, lid_c) * shell_k - (1.0 - open) * (1.0 - abs(lid_c)) * line_half;
  let d_thorns = mix(d_body, d_spikes, max(closing, abs(lid_c)));
  let d_shell = min(d_body, d_thorns);
  col = over(col, vec4<f32>(F.col_shell.rgb, aa(d_shell, px)));

  // Apertura dentro de la almendra, sin las púas. Un margen de medio píxel deja el
  // contorno delante y garantiza que con open = 0 no asome ningún círculo.
  let d_lid = mix(d_line, sd_eye_lens(s, opening), closing) * shell_k * bend_k + 0.5 * px;
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
  let q = rot2(-ang) * (p - center) / fscale;
  let w = q / EYE_R;                     // pantalla, unidades R, origen en el centro de la cabeza
  let pxu = px / (fscale * EYE_R);
  let voice = F.face2.y;
  let glow = F.face.x + voice * 0.5;
  let fxk = clamp(F.fx.x / FX_FULL, 0.0, 1.0);
  var col = vec4<f32>(0.0);

  // Volumen (yaw, pitch, roll): cada píxel del disco de la cabeza toca la esfera en un punto
  // delante y otro detrás; ambos se llevan al espacio de la cabeza deshaciendo el giro.
  let to_head = head_inverse(F.root2.y, F.head.x, F.head.y);
  let zs = sqrt(max(HEAD_R * HEAD_R - dot(w, w), 0.0));
  let head = aa(length(w) - HEAD_R, pxu);
  let front = to_head * vec3<f32>(w, zs);
  let back = to_head * vec3<f32>(w, -zs);
  let pxf = pxu * head_stretch(to_head, w, zs);
  let pxb = pxu * head_stretch(to_head, w, -zs);

  // De espalda: la placa se ve a través de la cabeza, verde liso y sin ojos. Provisional: cuando
  // la cabeza tape la cara, esta capa desaparece.
  let back_plate = aa(sd_box(back.xy, PLATE_HALF, PLATE_CORNER), pxb) * step(0.0, back.z) * head;
  col = over(col, vec4<f32>(F.col_mask.rgb, back_plate));

  // Mask + halo, en el hemisferio delantero de la cabeza
  let u = front.xy;                      // espacio de placa, unidades R
  let facing = step(0.0, front.z) * head;
  let dm = sd_box(u, PLATE_HALF, PLATE_CORNER);
  let plate = aa(dm, pxf) * facing;
  let halo = exp(-max(dm, 0.0) * 1.4) * glow * 0.45 * fxk * facing;
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
    let eye = eye_layer(e, size, gaze * mirror, open, F.face2.z, pxf);
    col = over(col, vec4<f32>(eye.rgb, eye.a * plate)); // recortado a la placa (ojos ampliados incluidos)
  }
  return col;
}
