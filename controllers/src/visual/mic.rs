//! Micrófono de prueba: volumen del micrófono por defecto → `voice.level` (0..1).
//!
//! El hilo de audio solo guarda el pico de RMS desde la última lectura; la conversión a nivel
//! (decibelios, puerta de ruido, ataque y caída) se hace en el hilo de la interfaz, así el
//! callback de audio no reserva memoria ni bloquea.
//!
//! Los errores del stream tampoco se imprimen desde el hilo de audio (escribir en la consola de
//! Windows es lento y retrasa la lectura, lo que provoca más desbordes): se clasifican y se dejan
//! en atómicos para que la interfaz los muestre y, si el stream cae, lo vuelva a abrir.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock, mpsc};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{ErrorKind, FromSample, SampleFormat, SizedSample};

/// Rango de decibelios que va de nivel 0 a 1, sobre el umbral que fija la sensibilidad.
const RANGE_DB: f32 = 30.0;
/// Umbral (dBFS) con sensibilidad 0 y con sensibilidad 1.
const FLOOR_DB: (f32, f32) = (-30.0, -65.0);
/// Suavizado por lectura (~30 Hz): sube rápido y baja algo más lento, como una boca.
const ATTACK: f32 = 0.7;
const RELEASE: f32 = 0.35;
/// Espera entre intentos de abrir el micrófono cuando no hay o se ha caído.
pub const RETRY: Duration = Duration::from_secs(2);
/// Como mucho una línea de resumen de xruns en la consola cada este tiempo.
const XRUN_LOG: Duration = Duration::from_secs(10);

/// Qué hacer con un error del stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    /// Desborde del búfer: se pierden unas muestras, inofensivo para medir volumen. Se cuenta.
    Xrun,
    /// El sistema re-enrutó el stream a otro dispositivo; sigue funcionando.
    Rerouted,
    /// Aviso sin consecuencias para el stream (p. ej. sin prioridad de tiempo real).
    Ignored,
    /// El stream ha muerto o ya no escucha el micrófono por defecto: hay que reabrirlo.
    Fatal,
}

fn classify(kind: ErrorKind) -> Fault {
    match kind {
        ErrorKind::Xrun => Fault::Xrun,
        ErrorKind::DeviceChanged => Fault::Rerouted,
        ErrorKind::RealtimeDenied => Fault::Ignored,
        // WASAPI avisa del cambio de micrófono por defecto con `StreamInvalidated` (el stream
        // sigue en el dispositivo viejo); el resto (desconectado, ocupado, permiso…) lo mata.
        _ => Fault::Fatal,
    }
}

/// Motivo legible de un error fatal.
fn describe(error: &cpal::Error) -> String {
    match error.kind() {
        ErrorKind::DeviceNotAvailable => "micrófono desconectado o no disponible".into(),
        ErrorKind::DeviceBusy => "micrófono ocupado por otra aplicación".into(),
        ErrorKind::StreamInvalidated => "cambió el micrófono por defecto".into(),
        ErrorKind::PermissionDenied => "permiso denegado (privacidad de Windows)".into(),
        _ => error.to_string(),
    }
}

/// Lo que el hilo de audio comunica a la interfaz, sin bloqueos.
#[derive(Default)]
struct Shared {
    /// Pico de RMS desde la última lectura (bits de un f32 positivo).
    peak: AtomicU32,
    xruns: AtomicU32,
    rerouted: AtomicBool,
    /// Primer error fatal; se escribe una vez y el stream se da por caído.
    fatal: OnceLock<String>,
}

impl Shared {
    fn report(&self, error: cpal::Error) {
        match classify(error.kind()) {
            Fault::Xrun => {
                self.xruns.fetch_add(1, Ordering::Relaxed);
            }
            Fault::Rerouted => self.rerouted.store(true, Ordering::Relaxed),
            Fault::Ignored => {}
            Fault::Fatal => {
                let _ = self.fatal.set(describe(&error));
            }
        }
    }
}

pub struct Mic {
    _stream: cpal::Stream,
    shared: Arc<Shared>,
    level: f32,
    xrun_log: XrunLog,
    pub device: String,
    /// Ha llegado alguna muestra distinta de cero. Si no, el dispositivo entrega silencio
    /// digital: micrófono silenciado o nada conectado a la entrada.
    pub heard: bool,
}

impl Mic {
    pub fn start() -> Result<Self, String> {
        let device = cpal::default_host().default_input_device().ok_or("no hay micrófono por defecto")?;
        let config = device.default_input_config().map_err(|e| e.to_string())?;
        // Búfer por defecto: con WASAPI los xruns de entrada los marca el driver (salto en la
        // captura) y pedir más búfer no los reduce; se cuentan y ya.
        let shared = Arc::new(Shared::default());
        let stream = open(&device, config.sample_format(), config.config(), &shared)?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self { _stream: stream, shared, level: 0.0, xrun_log: XrunLog::default(), device: device.to_string(), heard: false })
    }

    /// Lee el pico desde la última llamada y devuelve el nivel suavizado (0..1).
    /// `sensitivity` 0..1: más alto = reacciona a sonidos más bajos.
    pub fn level(&mut self, sensitivity: f32) -> f32 {
        let rms = f32::from_bits(self.shared.peak.swap(0, Ordering::Relaxed));
        self.heard |= rms > 0.0;
        let target = to_level(rms, sensitivity);
        let k = if target > self.level { ATTACK } else { RELEASE };
        self.level += (target - self.level) * k;
        self.level
    }

    /// Desbordes acumulados desde que se abrió el stream.
    pub fn xruns(&self) -> u32 {
        self.shared.xruns.load(Ordering::Relaxed)
    }

    /// Motivo si el stream ha caído y hay que reabrirlo.
    pub fn fault(&self) -> Option<&str> {
        self.shared.fatal.get().map(String::as_str)
    }

    /// El sistema ha re-enrutado el stream desde la última consulta.
    pub fn take_rerouted(&self) -> bool {
        self.shared.rerouted.swap(false, Ordering::Relaxed)
    }

    /// Línea de resumen de xruns para la consola, como mucho una cada `XRUN_LOG`.
    pub fn xrun_summary(&mut self, now: Instant) -> Option<String> {
        let total = self.xruns();
        self.xrun_log.due(total, now).then(|| format!("[mic] {total} desbordes de búfer acumulados (inofensivos para el volumen)"))
    }
}

/// Apertura en segundo plano: abrir el dispositivo puede tardar y no debe congelar la interfaz.
pub struct Opening(mpsc::Receiver<Result<Mic, String>>);

impl Opening {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();
        // Si la interfaz ya no espera (casilla desactivada), el envío falla y el Mic se suelta.
        std::thread::spawn(move || drop(tx.send(Mic::start())));
        Self(rx)
    }

    /// `None` mientras sigue abriendo.
    pub fn poll(&self) -> Option<Result<Mic, String>> {
        match self.0.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("falló el hilo de apertura".into())),
        }
    }
}

/// Toca volver a intentar abrir: nunca se ha intentado o pasó `RETRY` desde el último fallo.
pub fn retry_due(failed_at: Option<Instant>, now: Instant) -> bool {
    failed_at.is_none_or(|t| now.saturating_duration_since(t) >= RETRY)
}

/// Cuándo imprimir el resumen de xruns: solo si hay nuevos y como mucho uno cada `XRUN_LOG`.
#[derive(Default)]
struct XrunLog {
    reported: u32,
    at: Option<Instant>,
}

impl XrunLog {
    fn due(&mut self, total: u32, now: Instant) -> bool {
        let quiet = self.at.is_none_or(|t| now.saturating_duration_since(t) >= XRUN_LOG);
        if total <= self.reported || !quiet {
            return false;
        }
        (self.reported, self.at) = (total, Some(now));
        true
    }
}

fn open(device: &cpal::Device, format: SampleFormat, config: cpal::StreamConfig, shared: &Arc<Shared>) -> Result<cpal::Stream, String> {
    match format {
        SampleFormat::F32 => build::<f32>(device, config, shared.clone()),
        SampleFormat::I16 => build::<i16>(device, config, shared.clone()),
        SampleFormat::I32 => build::<i32>(device, config, shared.clone()),
        SampleFormat::U16 => build::<u16>(device, config, shared.clone()),
        SampleFormat::I8 => build::<i8>(device, config, shared.clone()),
        other => Err(format!("formato de audio no soportado: {other}")),
    }
}

fn build<T>(device: &cpal::Device, config: cpal::StreamConfig, shared: Arc<Shared>) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let errors = shared.clone();
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &_| {
                let rms = rms(data.iter().map(|s| s.to_sample::<f32>()));
                // Los f32 positivos ordenan igual que sus bits: el máximo se guarda sin bloqueo.
                shared.peak.fetch_max(rms.to_bits(), Ordering::Relaxed);
            },
            move |e| errors.report(e),
            None,
        )
        .map_err(|e| describe(&e))
}

fn rms(samples: impl Iterator<Item = f32>) -> f32 {
    let (sum, n) = samples.fold((0.0, 0u32), |(s, n), x| (s + x * x, n + 1));
    if n == 0 { 0.0 } else { (sum / n as f32).sqrt() }
}

/// RMS (0..1 a fondo de escala) → nivel 0..1 en escala de decibelios.
fn to_level(rms: f32, sensitivity: f32) -> f32 {
    let s = sensitivity.clamp(0.0, 1.0);
    let floor = FLOOR_DB.0 + (FLOOR_DB.1 - FLOOR_DB.0) * s;
    let db = 20.0 * rms.max(1e-6).log10();
    ((db - floor) / RANGE_DB).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_zero_and_loud_is_one() {
        assert_eq!(to_level(0.0, 0.5), 0.0);
        assert_eq!(to_level(1.0, 0.5), 1.0);
        // Más sensibilidad = el mismo sonido abre más la boca.
        assert!(to_level(0.01, 1.0) > to_level(0.01, 0.0));
    }

    /// Manual: `cargo test -p capri-ctl --features visual --bin capri-eyes -- --ignored mic --nocapture`.
    #[test]
    #[ignore = "necesita un micrófono real"]
    fn opens_the_default_microphone() {
        let mut mic = Mic::start().expect("micrófono");
        let (mut max, mut raw) = (0.0f32, 0.0f32);
        for _ in 0..60 {
            std::thread::sleep(std::time::Duration::from_millis(33));
            raw = raw.max(f32::from_bits(mic.shared.peak.load(Ordering::Relaxed)));
            max = max.max(mic.level(1.0));
        }
        println!(
            "{}: en 2 s, RMS máx {raw:.6} ({:.1} dBFS), nivel máx {max:.2}, xruns {}, fallo {:?}",
            mic.device,
            20.0 * raw.max(1e-9).log10(),
            mic.xruns(),
            mic.fault()
        );
    }

    #[test]
    fn rms_of_a_square_wave_is_its_amplitude() {
        assert!((rms([0.5, -0.5, 0.5, -0.5].into_iter()) - 0.5).abs() < 1e-6);
        assert_eq!(rms(std::iter::empty()), 0.0);
    }

    #[test]
    fn errors_are_classified() {
        assert_eq!(classify(ErrorKind::Xrun), Fault::Xrun);
        assert_eq!(classify(ErrorKind::DeviceChanged), Fault::Rerouted);
        assert_eq!(classify(ErrorKind::RealtimeDenied), Fault::Ignored);
        for kind in [ErrorKind::DeviceNotAvailable, ErrorKind::DeviceBusy, ErrorKind::StreamInvalidated, ErrorKind::BackendError] {
            assert_eq!(classify(kind), Fault::Fatal, "{kind:?}");
        }
    }

    #[test]
    fn xruns_are_counted_and_the_first_fatal_error_wins() {
        let shared = Shared::default();
        for _ in 0..500 {
            shared.report(ErrorKind::Xrun.into());
        }
        shared.report(ErrorKind::DeviceChanged.into());
        assert_eq!(shared.xruns.load(Ordering::Relaxed), 500);
        assert!(shared.rerouted.load(Ordering::Relaxed));
        assert!(shared.fatal.get().is_none(), "ni xruns ni re-enrutado tumban el stream");
        shared.report(ErrorKind::DeviceNotAvailable.into());
        shared.report(ErrorKind::DeviceBusy.into());
        assert_eq!(shared.fatal.get().map(String::as_str), Some("micrófono desconectado o no disponible"));
    }

    #[test]
    fn retries_every_two_seconds() {
        let t = Instant::now();
        assert!(retry_due(None, t));
        assert!(!retry_due(Some(t), t + Duration::from_millis(1999)));
        assert!(retry_due(Some(t), t + RETRY));
    }

    #[test]
    fn xrun_summary_is_occasional() {
        let (mut log, t) = (XrunLog::default(), Instant::now());
        assert!(!log.due(0, t), "sin xruns no hay línea");
        assert!(log.due(3, t), "el primero se avisa enseguida");
        assert!(!log.due(900, t + Duration::from_secs(9)), "no más de una línea cada 10 s");
        assert!(log.due(900, t + XRUN_LOG));
        assert!(!log.due(900, t + Duration::from_secs(30)), "sin nuevos no se repite");
    }
}
