//! capri: el cuerpo. Ventana transparente, sin bordes, siempre encima y click-through.
//!
//! ```text
//! capri [--port 7777] [--size 360x520] [--style assets/styles/construct.json]
//!       [--interactive]   la ventana recibe el ratón (para depurar)
//!       [--no-top]        no forzar siempre encima
//! ```
//! Se cierra con Ctrl+C en la terminal (la ventana no recibe clics a propósito).

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use capri_core::State;
use capri_protocol::{Envelope, DEFAULT_PORT};
use capri_render::{Frame, Renderer, StyleFile};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId, WindowLevel};

/// Frame mientras algo se mueve.
const ACTIVE: Duration = Duration::from_millis(16);
/// Frame en reposo: solo respiración y parpadeo.
const IDLE: Duration = Duration::from_millis(33);
const STYLE_POLL: Duration = Duration::from_secs(1);

struct Args {
    port: u16,
    width: u32,
    height: u32,
    style: String,
    click_through: bool,
    on_top: bool,
}

impl Args {
    fn parse() -> Self {
        let mut a = Args {
            port: DEFAULT_PORT,
            width: 360,
            height: 520,
            style: "assets/styles/construct.json".into(),
            click_through: true,
            on_top: true,
        };
        let mut it = std::env::args().skip(1);
        while let Some(flag) = it.next() {
            match flag.as_str() {
                "--port" => a.port = it.next().and_then(|v| v.parse().ok()).unwrap_or(a.port),
                "--size" => {
                    if let Some((w, h)) = it.next().as_deref().and_then(|v| v.split_once('x')) {
                        a.width = w.parse().unwrap_or(a.width);
                        a.height = h.parse().unwrap_or(a.height);
                    }
                }
                "--style" => a.style = it.next().unwrap_or(a.style),
                "--interactive" => a.click_through = false,
                "--no-top" => a.on_top = false,
                other => eprintln!("[capri] opción desconocida: {other}"),
            }
        }
        a
    }
}

struct Wake;

struct App {
    args: Args,
    rx: Receiver<Envelope>,
    state: State,
    style: StyleFile,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    last_frame: Instant,
    last_style_poll: Instant,
}

impl App {
    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) else { return };
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        let mut got_data = false;
        for env in self.rx.try_iter() {
            self.state.apply(&env, now);
            got_data = true;
        }
        if now.duration_since(self.last_style_poll) >= STYLE_POLL {
            self.last_style_poll = now;
            self.style.reload_if_changed();
        }
        let moving = self.state.tick(dt, now) || got_data;

        let (w, h) = renderer.size();
        let frame = Frame::build(&self.state, &self.style.style, w, h, renderer.premultiplied());
        window.pre_present_notify();
        renderer.render(&frame);

        let next = if moving { ACTIVE } else { IDLE };
        event_loop.set_control_flow(ControlFlow::WaitUntil(now + next));
    }
}

impl ApplicationHandler<Wake> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Capri")
            .with_transparent(true)
            .with_decorations(false)
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(self.args.width, self.args.height))
            .with_window_level(if self.args.on_top { WindowLevel::AlwaysOnTop } else { WindowLevel::Normal });
        // Windows: sin bitmap de redirección, si no DWM pinta un fondo opaco bajo la capa DirectComposition.
        #[cfg(windows)]
        let attrs = {
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs.with_no_redirection_bitmap(true)
        };
        let window = Arc::new(event_loop.create_window(attrs).expect("no se pudo crear la ventana"));

        // Esquina inferior derecha del monitor actual.
        if let Some(m) = window.current_monitor() {
            let (mp, ms, ws) = (m.position(), m.size(), window.outer_size());
            window.set_outer_position(PhysicalPosition::new(
                mp.x + ms.width as i32 - ws.width as i32 - 32,
                mp.y + ms.height as i32 - ws.height as i32 - 64,
            ));
        }
        if self.args.click_through {
            if let Err(e) = window.set_cursor_hittest(false) {
                eprintln!("[capri] click-through no soportado aquí: {e}");
            }
        }

        let size = window.inner_size();
        match Renderer::new(window.clone(), size.width, size.height) {
            Ok(r) => self.renderer = Some(r),
            Err(e) => {
                eprintln!("[capri] GPU: {e}");
                event_loop.exit();
                return;
            }
        }
        self.last_frame = Instant::now();
        window.request_redraw();
        self.window = Some(window);
    }

    fn new_events(&mut self, _: &ActiveEventLoop, cause: StartCause) {
        if let (StartCause::ResumeTimeReached { .. }, Some(w)) = (cause, &self.window) {
            w.request_redraw();
        }
    }

    fn user_event(&mut self, _: &ActiveEventLoop, _: Wake) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(s) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(s.width, s.height);
                }
            }
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }
}

fn main() {
    let args = Args::parse();
    let event_loop = EventLoop::<Wake>::with_user_event().build().expect("event loop");
    let proxy = Mutex::new(event_loop.create_proxy());

    let (tx, rx) = mpsc::channel();
    let addr = format!("127.0.0.1:{}", args.port);
    if let Err(e) = capri_link::spawn(&addr, tx, move || {
        let _ = proxy.lock().map(|p| p.send_event(Wake));
    }) {
        eprintln!("[capri] no se pudo abrir {addr}: {e}");
        return;
    }

    let mut app = App {
        style: StyleFile::open(&args.style),
        args,
        rx,
        state: State::default(),
        window: None,
        renderer: None,
        last_frame: Instant::now(),
        last_style_poll: Instant::now(),
    };
    event_loop.run_app(&mut app).expect("loop");
}
