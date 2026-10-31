//! Ventana nativa de prueba; solo envía protocolo al proceso Capri por WebSocket.

mod connection;
mod eyes;
mod process;

use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Color32, RichText};

use connection::{Connection, Event};
use eyes::Eyes;
use process::Preview;

const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

struct Args {
    spawn: bool,
    port: Option<u16>,
    style: PathBuf,
}

impl Args {
    fn parse() -> Result<Option<Self>, String> {
        let mut args = Self {
            spawn: false,
            port: None,
            style: PathBuf::from(REPO).join("assets/styles/capri-eyes.json"),
        };
        let mut input = std::env::args().skip(1);
        while let Some(arg) = input.next() {
            match arg.as_str() {
                "--spawn" => args.spawn = true,
                "--port" => {
                    let port = input
                        .next()
                        .ok_or("falta el puerto")?
                        .parse::<u16>()
                        .map_err(|_| "puerto inválido")?;
                    if port == 0 {
                        return Err("el puerto debe ser mayor que cero".into());
                    }
                    args.port = Some(port);
                }
                "--style" => args.style = input.next().ok_or("falta el estilo")?.into(),
                "--help" | "-h" => {
                    println!(
                        "capri-eyes [--spawn] [--port 7777] [--style archivo.json]\n\
                              --spawn abre y administra un Capri de prueba (puerto libre por defecto).\n\
                              Sin --spawn controla un Capri existente en el puerto indicado."
                    );
                    return Ok(None);
                }
                _ => return Err(format!("opción desconocida: {arg}")),
            }
        }
        Ok(Some(args))
    }
}

struct Controller {
    eyes: Eyes,
    linked: bool,
    connection: Connection,
    preview: Option<Preview>,
    port: u16,
    connected: bool,
    status: String,
    sent: String,
    notice: String,
    style_path: Option<PathBuf>,
    style_text: String,
    last_style: Instant,
}

impl Controller {
    fn new(port: u16, preview: Option<Preview>, style_path: Option<PathBuf>) -> Self {
        let eyes = Eyes::default();
        let connection = Connection::start(port, eyes.json());
        Self {
            eyes,
            connection,
            preview,
            style_path,
            port,
            linked: true,
            connected: false,
            status: "Conectando…".into(),
            sent: String::new(),
            notice: String::new(),
            style_text: String::new(),
            last_style: Instant::now() - Duration::from_secs(2),
        }
    }

    fn save(&mut self) {
        let result = (|| -> Result<PathBuf, Box<dyn std::error::Error>> {
            let dir = PathBuf::from(REPO).join("controllers/captures");
            std::fs::create_dir_all(&dir)?;
            let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
            let path = dir.join(format!("eyes-{stamp}.jsonl"));
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            file.write_all(self.eyes.scenario().as_bytes())?;
            Ok(path)
        })();
        self.notice = match result {
            Ok(path) => format!("Ajuste guardado: {}", path.display()),
            Err(error) => format!("No se pudo guardar: {error}"),
        };
    }

    fn eye_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Párpados y tamaño");
        ui.checkbox(&mut self.linked, "Vincular ojos al mover un control");
        ui.horizontal_wrapped(|ui| {
            for (label, open) in [
                ("Abrir", [1.0, 1.0]),
                ("A medias", [0.5, 0.5]),
                ("Guiño izq.", [0.0, 1.0]),
                ("Guiño der.", [1.0, 0.0]),
            ] {
                if ui.button(label).clicked() {
                    self.eyes.open = open;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Cerrar:");
            for (label, lid) in [("Neutro", 0.0), ("Feliz", 1.0), ("Triste", -1.0)] {
                if ui.button(label).clicked() {
                    self.eyes.open = [0.0, 0.0];
                    self.eyes.lid = lid;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.monospace("face → eye.lid");
            ui.add(
                egui::Slider::new(&mut self.eyes.lid, -1.0..=1.0)
                    .step_by(0.01)
                    .fixed_decimals(2)
                    .text("Curva al cerrar · -1 triste / 1 feliz"),
            );
        });
        ui.add_space(8.0);
        ui.columns(2, |columns| {
            for (i, ui) in columns.iter_mut().enumerate() {
                let side = if i == 0 { "l" } else { "r" };
                ui.strong(if i == 0 {
                    "Ojo izquierdo"
                } else {
                    "Ojo derecho"
                });
                ui.label("Apertura · 0 cerrado / 1 abierto");
                ui.monospace(format!("face → eye.{side}.open"));
                if ui
                    .add(
                        egui::Slider::new(&mut self.eyes.open[i], 0.0..=1.0)
                            .step_by(0.01)
                            .fixed_decimals(2),
                    )
                    .changed()
                    && self.linked
                {
                    self.eyes.open[1 - i] = self.eyes.open[i];
                }
                ui.add_space(8.0);
                ui.label("Tamaño · 1 normal");
                ui.monospace(format!("face → eye.{side}.size"));
                if ui
                    .add(
                        egui::Slider::new(&mut self.eyes.size[i], 0.5..=1.5)
                            .step_by(0.01)
                            .fixed_decimals(2),
                    )
                    .changed()
                    && self.linked
                {
                    self.eyes.size[1 - i] = self.eyes.size[i];
                }
            }
        });
        ui.add_space(8.0);
        ui.small("Izquierda y derecha como se ven en pantalla. El parpadeo automático de Capri sigue activo.");
    }

    fn gaze_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Mirada");
        ui.horizontal(|ui| {
            gaze_pad(ui, &mut self.eyes.gaze);
            ui.vertical(|ui| {
                for (i, name) in ["face → gaze.x", "face → gaze.y"].into_iter().enumerate() {
                    ui.monospace(name);
                    ui.add(
                        egui::Slider::new(&mut self.eyes.gaze[i], -1.0..=1.0)
                            .step_by(0.01)
                            .fixed_decimals(2),
                    );
                }
                ui.small("Arrastra dentro del círculo; doble clic centra.");
            });
        });
        ui.horizontal_wrapped(|ui| {
            for (label, gaze) in [
                ("Izquierda", [-1.0, 0.0]),
                ("Centro", [0.0, 0.0]),
                ("Derecha", [1.0, 0.0]),
                ("Arriba", [0.0, 1.0]),
                ("Abajo", [0.0, -1.0]),
            ] {
                if ui.button(label).clicked() {
                    self.eyes.gaze = gaze;
                }
            }
        });
        // Los sliders también quedan limitados al círculo.
        clamp_to_circle(&mut self.eyes.gaze);
        ui.small(
            "Mueve juntos el anillo, el azul y el centro. El círculo central marca el límite dentro del óvalo.",
        );
    }

    fn yaw_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Giro (volumen)");
        ui.horizontal(|ui| {
            ui.monospace("body → yaw");
            ui.add(
                egui::Slider::new(&mut self.eyes.yaw, -180.0..=180.0)
                    .step_by(0.5)
                    .fixed_decimals(1)
                    .suffix("°"),
            );
        });
        for (angle, label) in [
            (0.0, "Frontal"),
            (22.5, "Giro leve"),
            (45.0, "Tres cuartos"),
            (67.5, "Casi perfil"),
            (90.0, "Perfil completo"),
            (135.0, "Tres cuartos trasero"),
        ] {
            ui.horizontal(|ui| {
                ui.label(format!("{label}:"));
                for sign in if angle == 0.0 { &[1.0][..] } else { &[-1.0, 1.0][..] } {
                    let value = angle * sign;
                    if ui.selectable_label(self.eyes.yaw == value, format!("{value}°")).clicked() {
                        self.eyes.yaw = value;
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            ui.label("Espalda:");
            if ui.selectable_label(self.eyes.yaw == 180.0, "180°").clicked() {
                self.eyes.yaw = 180.0;
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.monospace("body → pitch");
            ui.add(
                egui::Slider::new(&mut self.eyes.pitch, -90.0..=90.0)
                    .step_by(0.5)
                    .fixed_decimals(1)
                    .suffix("°")
                    .text("Arriba / abajo"),
            );
        });
        ui.horizontal(|ui| {
            ui.monospace("body → roll");
            ui.add(
                egui::Slider::new(&mut self.eyes.roll, -90.0..=90.0)
                    .step_by(0.5)
                    .fixed_decimals(1)
                    .suffix("°")
                    .text("Inclinación lateral"),
            );
        });
        ui.horizontal_wrapped(|ui| {
            for (label, pitch, roll) in [
                ("Arriba", 30.0, 0.0),
                ("Abajo", -30.0, 0.0),
                ("Inclinar izq.", 0.0, 20.0),
                ("Inclinar der.", 0.0, -20.0),
                ("Recto", 0.0, 0.0),
            ] {
                if ui.button(label).clicked() {
                    self.eyes.pitch = pitch;
                    self.eyes.roll = roll;
                }
            }
        });
        ui.small("La placa gira sobre una esfera. De espalda se ve verde, sin ojos (provisional).");
    }

    fn context_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Contexto de la prueba");
        ui.horizontal_wrapped(|ui| {
            ui.monospace("morph → core");
            for (core, label) in [("eyes", "Ojos"), ("full", "Cuerpo"), ("bolt", "Rayo")] {
                if ui
                    .selectable_value(&mut self.eyes.core, core, label)
                    .changed()
                {
                    self.eyes.scale = if core == "eyes" { 2.0 } else { 1.0 };
                }
            }
        });
        ui.horizontal(|ui| {
            ui.monospace("body → scale");
            ui.add(
                egui::Slider::new(&mut self.eyes.scale, 0.5..=2.5)
                    .step_by(0.05)
                    .fixed_decimals(2)
                    .text("Zoom"),
            );
        });
    }
}

impl eframe::App for Controller {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        for event in self.connection.events.try_iter() {
            match event {
                Event::Connected => {
                    self.connected = true;
                    self.status = "Conectado".into();
                }
                Event::Sent(json) => self.sent = json,
                Event::Disconnected(error) => {
                    self.connected = false;
                    self.status = format!("Sin conexión. Reintentando… {error}");
                }
            }
        }
        if self.preview.as_mut().is_some_and(Preview::exited) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.last_style.elapsed() >= Duration::from_secs(1) {
            self.last_style = Instant::now();
            if let Some(path) = &self.style_path {
                self.style_text = std::fs::read_to_string(path)
                    .unwrap_or_else(|e| format!("No se pudo leer: {e}"));
            }
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let before = self.eyes.clone();
        let mut resend = false;
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Capri / Ojos");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(if self.connected { "● CONECTADO" } else { "● SIN CONEXIÓN" })
                            .color(if self.connected { Color32::from_rgb(73, 211, 154) } else { Color32::from_rgb(240, 180, 90) }));
                    });
                });
                ui.label("Control visual · cambios en vivo en la ventana de Capri");
                ui.monospace(format!("ws://127.0.0.1:{}", self.port));
                if !self.connected { ui.small(&self.status); }
                ui.add_space(10.0);
                egui::Frame::group(ui.style()).show(ui, |ui| self.eye_controls(ui));
                ui.add_space(8.0);
                egui::Frame::group(ui.style()).show(ui, |ui| self.gaze_controls(ui));
                ui.add_space(8.0);
                egui::Frame::group(ui.style()).show(ui, |ui| self.yaw_controls(ui));
                ui.add_space(8.0);
                egui::Frame::group(ui.style()).show(ui, |ui| self.context_controls(ui));
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Restablecer").clicked() { self.eyes = Eyes::default(); }
                    resend = ui.button("Reaplicar").clicked();
                    if ui.button("Copiar JSON").clicked() {
                        ui.ctx().copy_text(serde_json::to_string_pretty(&self.eyes.messages()).unwrap());
                        self.notice = "JSON copiado al portapapeles".into();
                    }
                    if ui.add_enabled(self.connected && self.sent == self.eyes.json(),
                        egui::Button::new("Aprobar y guardar"))
                        .on_hover_text("Guarda los objetivos en controllers/captures como escenario .jsonl").clicked() {
                        self.save();
                    }
                });
                if !self.notice.is_empty() { ui.label(&self.notice); }
                ui.add_space(8.0);
                ui.collapsing("Parámetros / JSON de los objetivos", |ui| {
                    ui.small("Valores solicitados, sin TTL. El cuerpo los interpola con resortes; no son telemetría.");
                    let mut json = serde_json::to_string_pretty(&self.eyes.messages()).unwrap();
                    ui.add(egui::TextEdit::multiline(&mut json).code_editor()
                        .desired_width(f32::INFINITY).desired_rows(18).interactive(false));
                });
                ui.collapsing("Último envío", |ui| {
                    ui.small("Enviado por WebSocket; el protocolo no devuelve confirmación de aplicación.");
                    ui.monospace(if self.sent.is_empty() { "Todavía no se ha enviado ningún ajuste" } else { &self.sent });
                });
                if let Some(path) = &self.style_path {
                    ui.collapsing("Estilo / colores y efectos", |ui| {
                        ui.label(path.display().to_string());
                        ui.small("Edita este archivo en el IDE. Capri recarga sus colores y efectos al guardarlo.");
                        let mut text = self.style_text.clone();
                        ui.add(egui::TextEdit::multiline(&mut text).code_editor()
                            .desired_width(f32::INFINITY).interactive(false));
                    });
                }
            });
        });
        if self.eyes != before || resend {
            self.connection.set(self.eyes.json());
        }
    }
}

/// Mantiene la mirada dentro del círculo unitario y la redondea a 0.01.
fn clamp_to_circle(gaze: &mut [f32; 2]) {
    let len = (gaze[0] * gaze[0] + gaze[1] * gaze[1]).sqrt();
    if len > 1.0 {
        *gaze = gaze.map(|v| v / len);
    }
    // Truncar (no redondear) para no salirse del círculo.
    *gaze = gaze.map(|v| (v * 100.0).trunc() / 100.0);
}

/// Cuadro XY para la mirada: x a la derecha, y hacia arriba, ambos en -1..=1.
fn gaze_pad(ui: &mut egui::Ui, gaze: &mut [f32; 2]) {
    let size = egui::vec2(160.0, 160.0);
    let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
    let rect = response.rect;
    if response.double_clicked() {
        *gaze = [0.0, 0.0];
    } else if let Some(pos) = response.interact_pointer_pos() {
        let x = (pos.x - rect.center().x) / (rect.width() / 2.0);
        let y = (rect.center().y - pos.y) / (rect.height() / 2.0);
        *gaze = [x, y];
    }
    clamp_to_circle(gaze);

    let visuals = ui.visuals();
    let grid = egui::Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color);
    painter.rect_filled(rect, 4.0, visuals.extreme_bg_color);
    painter.line_segment([rect.center_top(), rect.center_bottom()], grid);
    painter.line_segment([rect.left_center(), rect.right_center()], grid);
    painter.circle_stroke(rect.center(), rect.width() / 2.0, grid);
    painter.rect_stroke(rect, 4.0, grid, egui::StrokeKind::Inside);

    let dot = egui::pos2(
        rect.center().x + gaze[0] * rect.width() / 2.0,
        rect.center().y - gaze[1] * rect.height() / 2.0,
    );
    let color = if response.hovered() || response.dragged() {
        Color32::WHITE
    } else {
        visuals.selection.bg_fill
    };
    painter.circle_filled(dot, 7.0, color);
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let (preview, port, style_path) = if args.spawn {
        let (preview, port) = Preview::spawn(args.port, &args.style)?;
        (Some(preview), port, Some(args.style))
    } else {
        (
            None,
            args.port.unwrap_or(capri_protocol::DEFAULT_PORT),
            None,
        )
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([680.0, 760.0])
            .with_min_inner_size([580.0, 480.0])
            .with_position([40.0, 40.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Capri — Control de ojos",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_theme(egui::Theme::Dark);
            let mut style = (*cc.egui_ctx.global_style()).clone();
            style.spacing.item_spacing = egui::vec2(8.0, 6.0);
            style.spacing.slider_width = 170.0;
            cc.egui_ctx.set_global_style(style);
            Ok(Box::new(Controller::new(port, preview, style_path)))
        }),
    )?;
    Ok(())
}

fn main() {
    let result = Args::parse().map_err(|e| e.into()).and_then(|args| {
        if let Some(args) = args {
            run(args)
        } else {
            Ok(())
        }
    });
    if let Err(error) = result {
        eprintln!("capri-eyes: {error}");
        std::process::exit(1);
    }
}
