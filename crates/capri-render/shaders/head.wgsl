// ─── Head: cara oscura de la cabeza ───────────────────────────────────────────
// La forma la proyecta la CPU (src/head.rs) cada vez que cambia el giro: piezas convexas en 3D
// (cuernos, corona, barbilla) ya giradas, cada una como su polígono convexo en pantalla.
// Aquí solo se pinta la unión. Unidades R, origen en el centro de la cabeza.
// El orden de campos DEBE coincidir con `HeadData` en src/head.rs.

struct Head {
  info: vec4<f32>,                // piezas, alcance, -, -
  parts: array<vec4<f32>, 32>,    // 2 por pieza: (primer vértice, vértices, -, -), caja (lo, hi)
  verts: array<vec4<f32>, 64>,    // vértices antihorarios, dos por vec4
};

@group(0) @binding(2) var<uniform> HD: Head;

const HEAD_PARTS: u32 = 16u;
const HEAD_VERTS: u32 = 128u;

fn head_vertex(i: u32) -> vec2<f32> {
  let v = HD.verts[min(i, HEAD_VERTS - 1u) / 2u];
  return select(v.xy, v.zw, (i & 1u) == 1u);
}

// Distancia con signo a la cara: unión (mínimo) de los polígonos convexos. Dentro de un polígono
// convexo antihorario todas las aristas dejan el punto a su izquierda.
// `w`: pantalla en unidades R, centro de la cabeza.
fn sd_head(w: vec2<f32>, px: f32) -> f32 {
  let n = min(u32(HD.info.x), HEAD_PARTS);
  if (length(w) > HD.info.y) {
    return 1e9;
  }
  var d = 1e9;
  for (var k = 0u; k < n; k++) {
    let box = HD.parts[2u * k + 1u];
    if (any(w < box.xy - px) || any(w > box.zw + px)) {
      continue;
    }
    let span = HD.parts[2u * k];
    let first = u32(span.x);
    let count = u32(span.y);
    var prev = head_vertex(first + count - 1u);
    var dd = 1e9;
    var inside = true;
    for (var i = 0u; i < count; i++) {
      let v = head_vertex(first + i);
      let e = v - prev;
      let q = w - prev;
      let b = q - e * clamp(dot(q, e) / max(dot(e, e), 1e-8), 0.0, 1.0);
      dd = min(dd, dot(b, b));
      inside = inside && (e.x * q.y - e.y * q.x >= 0.0);
      prev = v;
    }
    d = min(d, select(1.0, -1.0, inside) * sqrt(dd));
  }
  return d;
}
