// ─── FX: glitch y scanlines (los controla el preset gráfico) ──────────────────

// Desplazamiento horizontal de franjas al azar. y en 0..1 de la ventana.
fn glitch_offset(y: f32) -> f32 {
  let tick = floor(F.res.z * 14.0);
  let band = floor(y * 28.0);
  if (hash21(vec2<f32>(band, tick)) < F.fx.w) {
    return (hash21(vec2<f32>(tick, band + 9.0)) - 0.5) * 0.06;
  }
  return 0.0;
}

fn scanlines(y: f32) -> f32 {
  return 1.0 - F.fx.z * (0.5 + 0.5 * sin(y * F.fx.y * 6.2831853));
}
