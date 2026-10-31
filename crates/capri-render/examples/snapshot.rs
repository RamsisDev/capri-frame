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
//! ```
//! El centro de la cabeza cae siempre en el centro de la imagen (o de cada celda).

use capri_core::State;
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
                "--help" | "-h" => return Err("uso: snapshot <out.png> [--yaw D] [--pitch D] [--roll D] [--style capri-eyes] [--size 1200x720] [--core eyes] [--spikes 1] [--len 1] [--ppr N] [--bg 24242c] [--grid] [--yaws ..] [--pitches ..]".into()),
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

/// Estado del cuerpo ya asentado (sin resortes ni tiempo) para un giro dado.
fn pose(a: &Args, yaw: f32, pitch: f32, w: u32, h: u32) -> Result<State, String> {
    let mut state = State::default();
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
    (t.yaw.value, t.pitch.value, t.roll.value) = (yaw, pitch, a.roll);

    // Escala y posición para que R mida `ppr` píxeles y el centro de la cabeza quede en el centro.
    // Igual que main.wgsl: centro y escala de la cara según los pesos de los núcleos.
    let wsum = wf + we + wb;
    let head_y = 0.245 + 0.006 * state.idle.breath; // skeleton().head.y de silhouette.wgsl
    let center_y = (head_y * wf + 0.02 * we + 0.27 * wb) / wsum;
    let fscale = (wf + 2.2 * we + 0.9 * wb) / wsum;
    // Por defecto entra el pelo entero (crecido con `--len`): ~44 R de ancho, de 8.5 R bajo la
    // cabeza a 16.5 R sobre ella. Se centra el pelo, no la cabeza.
    let grow = (a.spikes * a.len).max(0.4);
    let ppr = a.ppr.unwrap_or((w as f32 / (HAIR_W * grow)).min(h as f32 / (HAIR_H * grow)));
    let lift = if a.ppr.is_some() { 0.0 } else { HAIR_MID * grow * ppr / h as f32 };
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

fn run() -> Result<(), String> {
    let a = Args::parse()?;
    let style = load_style(&a.style)?;
    let (w, h) = a.size.unwrap_or(if a.grid { (480, 300) } else { (1200, 720) });
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
    let mut frame_for = |yaw, pitch| -> Result<Vec<u8>, String> {
        let state = pose(&a, yaw, pitch, w, h)?;
        let frame = Frame::build(&state, &style, w, h, true);
        Ok(shoot(&mut r, &target, &frame, a.bg))
    };

    if !a.grid {
        let rgb = frame_for(a.yaw, a.pitch)?;
        write_png(&a.out, w, h, &rgb)?;
        println!("{} · {w}x{h} · yaw {} pitch {} roll {}", a.out, a.yaw, a.pitch, a.roll);
        return Ok(());
    }

    // Hoja de contacto: columnas = yaw, filas = pitch; 1 px de separación entre celdas.
    let (cols, rows) = (a.yaws.len() as u32, a.pitches.len() as u32);
    let (gw, gh) = (cols * w + cols - 1, rows * h + rows - 1);
    let line = [0x50u8, 0x50, 0x5a];
    let mut sheet: Vec<u8> = line.iter().copied().cycle().take((gw * gh * 3) as usize).collect();
    for (j, &pitch) in a.pitches.iter().enumerate() {
        for (i, &yaw) in a.yaws.iter().enumerate() {
            let tile = frame_for(yaw, pitch)?;
            let (x0, y0) = (i as u32 * (w + 1), j as u32 * (h + 1));
            for y in 0..h {
                let dst = (((y0 + y) * gw + x0) * 3) as usize;
                sheet[dst..dst + (w * 3) as usize].copy_from_slice(&tile[(y * w * 3) as usize..][..(w * 3) as usize]);
            }
        }
    }
    write_png(&a.out, gw, gh, &sheet)?;
    println!("{} · {gw}x{gh} · columnas yaw {:?} · filas pitch {:?} · roll {}", a.out, a.yaws, a.pitches, a.roll);
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("[snapshot] {e}");
        std::process::exit(1);
    }
}
