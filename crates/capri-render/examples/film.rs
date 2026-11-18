//! Render sin ventana de una animación: reproduce una línea de tiempo de mensajes del protocolo
//! y escribe los cuadros como RGB crudo por stdout (para entubar a ffmpeg).
//!
//! ```text
//! cargo run --release -p capri-render --example film -- <timeline.tsv> [opciones] | ffmpeg -f rawvideo ...
//!   --size WxH      tamaño del cuadro (1920x1080)
//!   --fps N         cuadros por segundo (60)
//!   --secs S        duración
//!   --ppr N         píxeles por R con `scale` = 1 (por defecto, la placa ocupa ~36 % del ancho en 16:9)
//!   --style NOMBRE  assets/styles/NOMBRE.json (capri-eyes)
//!   --bg RRGGBB     fondo (08080c)
//! ```
//! Cada línea de la línea de tiempo es `segundos<TAB>json`, con el JSON de docs/PROTOCOL.md.
//! Arranca en el núcleo `eyes` (placa, ojos y boca; sin pelo ni cuerpo) con la cabeza al centro.
//! `scale` y `pos` del stream `body` son relativos a ese encuadre.

use std::io::Write;
use std::time::{Duration, Instant};

use capri_core::body::Node;
use capri_core::State;
use capri_render::{Frame, Renderer, Style};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// R en unidades de cara (= `EYE_R` de face.wgsl).
const EYE_R: f32 = 0.0195;
/// Centro y escala de la cara en el núcleo `eyes` (HEAD_CORE_Y / HEAD_CORE_SCALE de main.wgsl).
const EYES_Y: f32 = 0.02;
const EYES_SCALE: f32 = 2.2;

fn apply(state: &mut State, json: &str, now: Instant) -> Result<(), String> {
    for env in capri_protocol::decode(json).map_err(|e| format!("{json}: {e}"))? {
        state.apply(&env, now);
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let timeline = args.next().ok_or("uso: film <timeline.tsv> [--size WxH] [--fps N] [--secs S] [--ppr N] [--style S] [--bg RRGGBB]")?;
    let (mut w, mut h, mut fps, mut secs, mut ppr) = (1920u32, 1080u32, 60u32, 0.0f64, None);
    let (mut style, mut bg) = ("capri-eyes".to_string(), "08080c".to_string());
    while let Some(flag) = args.next() {
        let v = args.next().ok_or(format!("falta el valor de {flag}"))?;
        let bad = || format!("valor inválido de {flag}: {v}");
        match flag.as_str() {
            "--size" => {
                let (a, b) = v.split_once('x').ok_or_else(bad)?;
                (w, h) = (a.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?);
            }
            "--fps" => fps = v.parse().map_err(|_| bad())?,
            "--secs" => secs = v.parse().map_err(|_| bad())?,
            "--ppr" => ppr = Some(v.parse::<f32>().map_err(|_| bad())?),
            "--style" => style = v,
            "--bg" => bg = v,
            _ => return Err(format!("opción desconocida: {flag}")),
        }
    }
    let ppr = ppr.unwrap_or(h as f32 * 0.85 / 11.0);
    let bg = capri_render::style::hex(&bg);

    let path = format!("{}/../../assets/styles/{style}.json", env!("CARGO_MANIFEST_DIR"));
    let style: Style = serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?)
        .map_err(|e| format!("{path}: {e}"))?;

    let text = std::fs::read_to_string(&timeline).map_err(|e| format!("{timeline}: {e}"))?;
    let mut events: Vec<(f64, &str)> = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
        let (t, json) = line.split_once('\t').ok_or(format!("línea sin tabulador: {line}"))?;
        events.push((t.trim().parse().map_err(|_| format!("tiempo inválido: {t}"))?, json));
    }
    events.sort_by(|a, b| a.0.total_cmp(&b.0));
    if secs <= 0.0 {
        secs = events.last().map_or(1.0, |e| e.0 + 1.0);
    }

    let mut r = Renderer::offscreen(w, h, FORMAT)?;
    let target = r.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("film"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let row = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = r.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (row * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Reloj falso: el tiempo de la película, no el del reloj de pared.
    let t0 = Instant::now();
    let mut state = State::default();
    apply(&mut state, r#"{"s":"morph","set":{"core":"eyes"}}"#, t0)?;
    state.body.visit(&mut |_, _, p| p.value = p.target());

    let base = ppr / (h as f32 * EYES_SCALE * EYE_R);
    let (dt, total) = (1.0 / fps as f32, (secs * fps as f64).round() as u64);
    let mut next = 0;
    let mut out = std::io::stdout().lock();
    let mut rgb = vec![0u8; (w * h * 3) as usize];
    for n in 0..total {
        let t = n as f64 / fps as f64;
        let now = t0 + Duration::from_secs_f64(t);
        while next < events.len() && events[next].0 <= t {
            apply(&mut state, events[next].1, now)?;
            next += 1;
        }
        state.tick(dt, now);

        // `scale` y `pos` relativos al encuadre base: se convierten solo para dibujar.
        let tr = &mut state.body.transform;
        let (scale, pos_y) = (tr.scale.value, tr.pos_y.value);
        tr.scale.value = scale * base;
        tr.pos_y.value = pos_y - 2.0 * EYES_Y * scale * base;
        let frame = Frame::build(&state, &style, w, h, true);
        let tr = &mut state.body.transform;
        (tr.scale.value, tr.pos_y.value) = (scale, pos_y);

        r.render_to(&frame, &view);
        let mut encoder = r.device().create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) },
            },
            target.size(),
        );
        r.queue().submit([encoder.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |res| res.expect("map"));
        r.device().poll(wgpu::PollType::wait_indefinitely()).expect("poll");
        {
            let data = buffer.slice(..).get_mapped_range().expect("mapped range");
            let mut dst = rgb.chunks_exact_mut(3);
            for y in 0..h {
                for px in data[(y * row) as usize..][..(w * 4) as usize].chunks_exact(4) {
                    // Salida premultiplicada: color + fondo * (1 - alfa).
                    let a = px[3] as f32 / 255.0;
                    let d = dst.next().expect("tamaño");
                    for c in 0..3 {
                        d[c] = (px[c] as f32 + bg[c] * 255.0 * (1.0 - a)).round().min(255.0) as u8;
                    }
                }
            }
        }
        buffer.unmap();
        out.write_all(&rgb).map_err(|e| e.to_string())?;
    }
    eprintln!("[film] {total} cuadros · {w}x{h} · {fps} fps");
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("[film] {e}");
        std::process::exit(1);
    }
}
