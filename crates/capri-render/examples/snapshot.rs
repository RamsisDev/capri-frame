//! Render sin ventana a PNG, para iterar la forma (sobre todo el pelo) sin abrir Capri.
//!
//! ```text
//! cargo run -p capri-render --example snapshot -- <out.png> [opciones]
//!   --yaw D --pitch D --roll D   giro de la cabeza en grados (0)
//!   --style NOMBRE|RUTA          assets/styles/NOMBRE.json o un .json (capri-eyes)
//!   --size WxH                   tamaño de la imagen; con --grid, de cada celda (1200x720 | 480x300)
//!   --core eyes|full|bolt        núcleo (eyes, como el panel capri-eyes)
//!   --spikes A --len L           pelo: presencia y largo, 1 = crecido del todo (1, 1)
//!   --ppr N                      píxeles por R (radio del anillo del ojo); por defecto, lo que deja entrar el pelo entero
//!   --bg RRGGBB                  fondo (24242c)
//!   --grid                       hoja de contacto: filas = pitch, columnas = yaw
//!   --yaws a,b,..  --pitches ..  valores de la hoja (-40,-20,0,20,40 × 15,0,-15)
//!   --expr NOMBRE                preset de cara ya asentado: neutral|focus|happy|angry|sad|surprised|sleep
//!   --viseme NOMBRE              x|mbp|a|e|i|o|u|fl (o "A", "F/L"): peso 1 a ese visema, 0 al resto
//!   --open V --curve V           mouth.open y mouth.curve crudos (después de --expr)
//!   --face                       encuadre de la cara (~13 × 11 R, cabeza al centro) en vez del pelo
//!   --mouth-grid                 hoja de bocas: columnas = visemas (x mbp a e i o u fl), filas = curve;
//!                                implica --face; tamaño de celda por defecto 300x260
//!   --curves a,b,..              filas de --mouth-grid (1,0,-1)
//! ```
//! El centro de la cabeza cae en el centro de la imagen (o de cada celda), salvo en el encuadre
//! del pelo sin `--ppr`, que centra el pelo. `--ppr` manda sobre cualquier encuadre.

use std::time::Instant;

use capri_core::body::{Face, Node, VISEMES};
use capri_core::{presets, State};
use capri_render::{Frame, Renderer, Style};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// R en unidades de cara (= `EYE_R` de face.wgsl).
const EYE_R: f32 = 0.0195;

struct Args {
    out: String,
    yaw: f32,
    pitch: f32,
    roll: f32,
    style: String,
    size: Option<(u32, u32)>,
    core: String,
    spikes: f32,
    len: f32,
    ppr: Option<f32>,
    bg: [f32; 3],
    grid: bool,
    yaws: Vec<f32>,
    pitches: Vec<f32>,
    /// Nombre (para la salida) y claves del preset de expresión.
    expr: Option<(String, Preset)>,
    viseme: Option<usize>,
    open: Option<f32>,
    curve: Option<f32>,
    face: bool,
    mouth_grid: bool,
    curves: Vec<f32>,
}

/// Lista de (clave, valor) de un preset de `capri_core::presets`.
type Preset = &'static [(&'static str, f32)];

/// Índice en `VISEMES` de un nombre como lo acepta el protocolo ("A", "mbp", "F/L").
fn viseme_index(name: &str) -> Result<usize, String> {
    let key = name.to_ascii_lowercase().replace('/', "");
    VISEMES.iter().position(|v| *v == key).ok_or(format!("visema desconocido: {name}"))
}

fn list(v: &str) -> Result<Vec<f32>, String> {
    v.split(',').map(|x| x.trim().parse().map_err(|_| format!("número inválido: {x}"))).collect()
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut a = Args {
            out: String::new(),
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            style: "capri-eyes".into(),
            size: None,
            core: "eyes".into(),
            spikes: 1.0,
            len: 1.0,
            ppr: None,
            bg: [0x24 as f32 / 255.0, 0x24 as f32 / 255.0, 0x2c as f32 / 255.0],
            grid: false,
            yaws: vec![-40.0, -20.0, 0.0, 20.0, 40.0],
            pitches: vec![15.0, 0.0, -15.0],
            expr: None,
            viseme: None,
            open: None,
            curve: None,
            face: false,
            mouth_grid: false,
            curves: vec![1.0, 0.0, -1.0],
        };
        let mut it = std::env::args().skip(1);
        while let Some(flag) = it.next() {
            let mut value = || it.next().ok_or(format!("falta el valor de {flag}"));
            let num = |v: String| v.parse::<f32>().map_err(|_| format!("número inválido: {v}"));
            match flag.as_str() {
                "--yaw" => a.yaw = num(value()?)?,
                "--pitch" => a.pitch = num(value()?)?,
                "--roll" => a.roll = num(value()?)?,
                "--style" => a.style = value()?,
                "--size" => {
                    let v = value()?;
                    let (w, h) = v.split_once('x').ok_or(format!("tamaño inválido: {v}"))?;
                    a.size = Some((w.parse().map_err(|_| "ancho inválido")?, h.parse().map_err(|_| "alto inválido")?));
                }
                "--core" => a.core = value()?,
                "--spikes" => a.spikes = num(value()?)?,
                "--len" => a.len = num(value()?)?,
                "--ppr" => a.ppr = Some(num(value()?)?),
                "--bg" => {
                    let c = capri_render::style::hex(&value()?);
                    a.bg = [c[0], c[1], c[2]];
                }
                "--grid" => a.grid = true,
                "--yaws" => a.yaws = list(&value()?)?,
                "--pitches" => a.pitches = list(&value()?)?,
                "--expr" => {
                    let v = value()?;
                    let p = presets::expr(&v).ok_or(format!("expresión desconocida: {v}"))?;
                    a.expr = Some((v, p));
                }
                "--viseme" => a.viseme = Some(viseme_index(&value()?)?),
                "--open" => a.open = Some(num(value()?)?),
                "--curve" => a.curve = Some(num(value()?)?),
                "--face" => a.face = true,
                "--mouth-grid" => (a.mouth_grid, a.face) = (true, true),
                "--curves" => a.curves = list(&value()?)?,
                "--help" | "-h" => return Err("uso: snapshot <out.png> [--yaw D] [--pitch D] [--roll D] [--style capri-eyes] [--size 1200x720] [--core eyes] [--spikes 1] [--len 1] [--ppr N] [--bg 24242c] [--grid] [--yaws ..] [--pitches ..] [--expr neutral|focus|happy|angry|sad|surprised|sleep] [--viseme x|mbp|a|e|i|o|u|fl] [--open V] [--curve V] [--face] [--mouth-grid] [--curves 1,0,-1]".into()),
                f if f.starts_with("--") => return Err(format!("opción desconocida: {f}")),
                _ if a.out.is_empty() => a.out = flag,
                _ => return Err(format!("argumento de más: {flag}")),
            }
        }
        if a.out.is_empty() {
            return Err("falta la ruta del PNG de salida".into());
        }
        Ok(a)
    }
}

fn load_style(name: &str) -> Result<Style, String> {
    let path = if name.ends_with(".json") || std::path::Path::new(name).is_file() {
        name.into()
    } else {
        format!("{}/../../assets/styles/{name}.json", env!("CARGO_MANIFEST_DIR"))
    };
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
}

/// Encuadre del pelo en unidades R: ancho y alto con margen, y altura de su centro sobre la cabeza.
const HAIR_W: f32 = 48.0;
const HAIR_H: f32 = 28.0;
const HAIR_MID: f32 = 4.0;
/// Encuadre de la cara en R: la cara oscura mide ~9 R de ancho, de y=+2.3 a y=-4.6.
const FACE_W: f32 = 13.0;
const FACE_H: f32 = 11.0;

/// Lo que cambia de una celda a otra: giro y, en la hoja de bocas, visema y curva.
#[derive(Clone, Copy)]
struct Cell {
    yaw: f32,
    pitch: f32,
    viseme: Option<usize>,
    curve: Option<f32>,
}

/// Fija claves de la cara (streams face y voice) ya asentadas: el snapshot no anima.
fn put(face: &mut Face, keys: &[(&str, f32)]) {
    let now = Instant::now();
    face.visit(&mut |_, key, p| {
        if let Some(&(_, v)) = keys.iter().find(|(k, _)| *k == key) {
            p.set(v, None, now);
            p.value = p.target();
        }
    });
}

/// Expresión y boca: --expr, luego el visema, luego los valores crudos.
fn mouth(a: &Args, cell: Cell, face: &mut Face) {
    if let Some((_, e)) = a.expr {
        put(face, e);
    }
    if let Some(p) = cell.viseme.and_then(|i| presets::viseme(VISEMES[i])) {
        put(face, p);
    }
    if let Some(v) = a.open {
        put(face, &[("mouth.open", v)]);
    }
    if let Some(v) = cell.curve {
        put(face, &[("mouth.curve", v)]);
    }
}

/// Píxeles por R y cuánto (en R) sube el centro del encuadre sobre la cabeza.
fn framing(a: &Args, w: u32, h: u32) -> (f32, f32) {
    let (w, h) = (w as f32, h as f32);
    if let Some(ppr) = a.ppr {
        return (ppr, 0.0);
    }
    if a.face {
        return ((w / FACE_W).min(h / FACE_H), 0.0);
    }
    // Pelo entero (crecido con `--len`): ~44 R de ancho, de 8.5 R bajo la cabeza a 16.5 R sobre
    // ella. Se centra el pelo, no la cabeza.
    let grow = (a.spikes * a.len).max(0.4);
    ((w / (HAIR_W * grow)).min(h / (HAIR_H * grow)), HAIR_MID * grow)
}

/// Estado del cuerpo ya asentado (sin resortes ni tiempo) para una celda.
fn pose(a: &Args, cell: Cell, w: u32, h: u32) -> Result<State, String> {
    let mut state = State::default();
    mouth(a, cell, &mut state.body.face);
    let b = &mut state.body;
    let (wf, we, wb) = match a.core.as_str() {
        "full" => (1.0, 0.0, 0.0),
        "eyes" => (0.0, 1.0, 0.0),
        "bolt" => (0.0, 0.0, 1.0),
        c => return Err(format!("núcleo desconocido: {c}")),
    };
    let m = &mut b.transform.shape_morph;
    (m.full.value, m.eyes.value, m.bolt.value) = (wf, we, wb);
    b.silhouette.opacity.value = if a.core == "eyes" { 0.0 } else { 1.0 };
    b.extensions.fragments.amount.value = 0.0;
    b.extensions.spikes.amount.value = a.spikes;
    b.extensions.spikes.length.value = a.len;
    let t = &mut b.transform;
    (t.yaw.value, t.pitch.value, t.roll.value) = (cell.yaw, cell.pitch, a.roll);

    // Escala y posición para que R mida `ppr` píxeles y el centro de la cabeza quede en el centro.
    // Igual que main.wgsl: centro y escala de la cara según los pesos de los núcleos.
    let wsum = wf + we + wb;
    let head_y = 0.245 + 0.006 * state.idle.breath; // skeleton().head.y de silhouette.wgsl
    let center_y = (head_y * wf + 0.02 * we + 0.27 * wb) / wsum;
    let fscale = (wf + 2.2 * we + 0.9 * wb) / wsum;
    let (ppr, lift_r) = framing(a, w, h);
    let lift = lift_r * ppr / h as f32;
    let scale = ppr / (h as f32 * fscale * EYE_R);
    let t = &mut state.body.transform;
    t.scale.value = scale;
    t.pos_y.value = -2.0 * center_y * scale - 2.0 * lift;
    Ok(state)
}

/// Dibuja un frame y lo devuelve como RGB sobre el fondo (`w * h * 3` bytes).
fn shoot(r: &mut Renderer, target: &wgpu::Texture, frame: &Frame, bg: [f32; 3]) -> Vec<u8> {
    let (w, h) = (target.width(), target.height());
    r.reset_hair(); // cada pose genera su pelo de cero, sin la histéresis de la anterior
    r.render_to(frame, &target.create_view(&Default::default()));
    let row = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = r.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (row * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
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
    let data = buffer.slice(..).get_mapped_range().expect("mapped range");
    let mut out = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for px in data[(y * row) as usize..][..(w * 4) as usize].as_chunks::<4>().0 {
            // Salida premultiplicada: color + fondo * (1 - alfa).
            let a = px[3] as f32 / 255.0;
            for c in 0..3 {
                out.push((px[c] as f32 + bg[c] * 255.0 * (1.0 - a)).round().min(255.0) as u8);
            }
        }
    }
    out
}

fn write_png(path: &str, w: u32, h: u32, rgb: &[u8]) -> Result<(), String> {
    if let Some(dir) = std::path::Path::new(path).parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let file = std::fs::File::create(path).map_err(|e| format!("{path}: {e}"))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgb).map_err(|e| e.to_string())
}

/// Celdas de la hoja (por filas) y número de columnas. Sin hoja, una sola celda.
fn layout(a: &Args) -> (Vec<Cell>, usize) {
    let base = Cell { yaw: a.yaw, pitch: a.pitch, viseme: a.viseme, curve: a.curve };
    if a.mouth_grid {
        let row = |c: f32| (0..VISEMES.len()).map(move |i| Cell { viseme: Some(i), curve: Some(c), ..base });
        return (a.curves.iter().flat_map(|&c| row(c)).collect(), VISEMES.len());
    }
    if a.grid {
        let row = |pitch: f32| a.yaws.iter().map(move |&yaw| Cell { yaw, pitch, ..base });
        return (a.pitches.iter().flat_map(|&p| row(p)).collect(), a.yaws.len());
    }
    (vec![base], 1)
}

/// Pega las celdas (por filas) en una hoja con 1 px de separación. Devuelve ancho, alto y RGB.
fn sheet(tiles: &[Vec<u8>], cols: usize, w: u32, h: u32) -> (u32, u32, Vec<u8>) {
    let (cols, rows) = (cols as u32, tiles.len().div_ceil(cols) as u32);
    let (gw, gh) = (cols * w + cols - 1, rows * h + rows - 1);
    let line = [0x50u8, 0x50, 0x5a];
    let mut out: Vec<u8> = line.iter().copied().cycle().take((gw * gh * 3) as usize).collect();
    for (n, tile) in tiles.iter().enumerate() {
        let (x0, y0) = (n as u32 % cols * (w + 1), n as u32 / cols * (h + 1));
        for y in 0..h {
            let dst = (((y0 + y) * gw + x0) * 3) as usize;
            out[dst..dst + (w * 3) as usize].copy_from_slice(&tile[(y * w * 3) as usize..][..(w * 3) as usize]);
        }
    }
    (gw, gh, out)
}

/// Qué muestra la imagen, para la línea de salida.
fn caption(a: &Args) -> String {
    let expr = a.expr.as_ref().map_or("-", |(name, _)| name.as_str());
    let face = format!("expr {expr} · open {:?}", a.open);
    let mouth = format!("visema {:?} · curve {:?}", a.viseme.map(|i| VISEMES[i]), a.curve);
    if a.mouth_grid {
        let cols = format!("columnas visema {VISEMES:?} · filas curve {:?}", a.curves);
        format!("{cols} · yaw {} pitch {} roll {} · {face}", a.yaw, a.pitch, a.roll)
    } else if a.grid {
        format!("columnas yaw {:?} · filas pitch {:?} · roll {} · {face} · {mouth}", a.yaws, a.pitches, a.roll)
    } else {
        format!("yaw {} pitch {} roll {} · {face} · {mouth}", a.yaw, a.pitch, a.roll)
    }
}

fn run() -> Result<(), String> {
    let a = Args::parse()?;
    let style = load_style(&a.style)?;
    let cell = if a.mouth_grid { (300, 260) } else if a.grid { (480, 300) } else { (1200, 720) };
    let (w, h) = a.size.unwrap_or(cell);
    let mut r = Renderer::offscreen(w, h, FORMAT)?;
    let target = r.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("snapshot"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    // Una celda o una hoja de contacto: el caso suelto es una hoja de 1 × 1.
    let (cells, cols) = layout(&a);
    let mut tiles = Vec::with_capacity(cells.len());
    for &cell in &cells {
        let state = pose(&a, cell, w, h)?;
        let frame = Frame::build(&state, &style, w, h, true);
        tiles.push(shoot(&mut r, &target, &frame, a.bg));
    }
    let (gw, gh, rgb) = sheet(&tiles, cols, w, h);
    write_png(&a.out, gw, gh, &rgb)?;
    println!("{} · {gw}x{gh} · {}", a.out, caption(&a));
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("[snapshot] {e}");
        std::process::exit(1);
    }
}
