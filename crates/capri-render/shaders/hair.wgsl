// ─── Hair: pelo procedural de la cabeza ───────────────────────────────────────
// La forma la genera la CPU (src/hair.rs) solo al cruzar un umbral de giro; aquí únicamente se
// pinta: triángulos planos de color `hair`, con manchas `hair_glint` concentradas en un foco.
// Dos capas: detrás de la cara y delante (flequillo). Unidades R, origen en el centro de la cabeza.
// El orden de campos DEBE coincidir con `HairData` en src/hair.rs.

struct Hair {
  info: vec4<f32>,                // triángulos, roll del tramo, alcance, -
  glint: vec4<f32>,               // foco x, foco y, radio del foco, semilla
  shards: array<vec4<f32>, 224>,  // 2 por triángulo: (a, b), (c, tono, capa)
};

@group(0) @binding(1) var<uniform> H: Hair;

const HAIR_MAX: u32 = 112u;

// Manchas: una capa de puntos en una rejilla girada `turn`. Cada celda decide si tiene punto según
// su cercanía al foco (polvo fino lejos, manchas densas cerca), así el patrón queda fijo sobre el
// pelo. Cada punto se corre al azar dentro de su celda y cabe entero en ella.
fn glint_dots(h: vec2<f32>, cell: f32, turn: f32, far: f32, near: f32, size: f32, px: f32) -> f32 {
  let shift = vec2<f32>(H.glint.w, H.glint.w * 1.7);
  let g = rot2(turn) * h / cell + shift;
  let id = floor(g);
  let mid = rot2(-turn) * ((id + 0.5 - shift) * cell);
  let q = (mid - H.glint.xy) / H.glint.z;
  let k = exp(-dot(q, q));
  if (hash21(id) > mix(far, near, k)) {
    return 0.0;
  }
  let c = id + 0.5 + (vec2<f32>(hash21(id + 13.1), hash21(id + 29.7)) - 0.5) * 0.5;
  let r = max(size * (0.5 + hash21(id + 5.3)) * (0.7 + 0.9 * k), 0.6 * px);
  return aa(length(g - c) * cell - r, px);
}

fn hair_glint(h: vec2<f32>, px: f32) -> f32 {
  var a = glint_dots(h, 0.4, 0.4, 0.06, 0.7, 0.045, px);
  a = max(a, glint_dots(h, 0.7, 1.1, 0.03, 0.6, 0.08, px));
  a = max(a, glint_dots(h, 1.3, 2.0, 0.0, 0.4, 0.15, px));
  return a;
}

struct HairLayers {
  back: vec4<f32>,   // detrás de la cara
  front: vec4<f32>,  // delante de la cara
};

// Pelo. `w`: pantalla en unidades R, centro de la cabeza, ya sin el giro del cuerpo. El roll sigue
// continuo entre umbrales; `spikes` y `spikes.len` hacen crecer el pelo.
fn hair_layer(w: vec2<f32>, pxu: f32) -> HairLayers {
  var out = HairLayers(vec4<f32>(0.0), vec4<f32>(0.0));
  let n = min(u32(H.info.x), HAIR_MAX);
  if (F.ext.x < 0.01 || n == 0u) {
    return out;
  }
  let grow = max(F.ext.x * F.ext.y, 1e-3);
  let h = rot2(-(F.head.y - H.info.y)) * w / grow;
  if (length(h) > H.info.z) {
    return out;
  }
  let px = pxu / grow;
  // Por capa: la silueta es la unión de los triángulos (distancia mínima, sin costuras entre
  // vecinos); el tono es el del último triángulo que cubre el píxel, así se ven las facetas.
  var d = vec2<f32>(1e9);
  var tone = vec2<f32>(1.0);
  for (var i = 0u; i < n; i++) {
    let s0 = H.shards[2u * i];
    let s1 = H.shards[2u * i + 1u];
    let lo = min(min(s0.xy, s0.zw), s1.xy) - px;
    let hi = max(max(s0.xy, s0.zw), s1.xy) + px;
    if (any(h < lo) || any(h > hi)) {
      continue;
    }
    let di = sd_triangle(h, s0.xy, s0.zw, s1.xy);
    let a = aa(di, px);
    if (s1.w > 0.5) {
      d.y = min(d.y, di);
      tone.y = mix(tone.y, s1.z, a);
    } else {
      d.x = min(d.x, di);
      tone.x = mix(tone.x, s1.z, a);
    }
  }
  let glint = hair_glint(h, px);
  let fade = smoothstep(0.0, 0.5, F.ext.x);
  out.back = vec4<f32>(mix(F.col_hair.rgb * tone.x, F.col_glint.rgb, glint), aa(d.x, px) * fade);
  out.front = vec4<f32>(mix(F.col_hair.rgb * tone.y, F.col_glint.rgb, glint), aa(d.y, px) * fade);
  return out;
}
