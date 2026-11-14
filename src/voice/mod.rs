pub mod parser;

use crate::roku::Cmd;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::Sender;
use std::thread;
// use vosk::{Model, Recognizer}; // Descomentar cuando agregues vosk a Cargo.toml

pub struct VoiceManager {
    cmd_tx: Sender<(String, Cmd)>,
}

impl VoiceManager {
    pub fn new(cmd_tx: Sender<(String, Cmd)>) -> Self {
        let manager = Self { cmd_tx };
        manager.start_audio_listener();
        manager
    }

    pub fn handle_spoken_text(&self, ip: &str, text: &str) -> bool {
        if let Some(cmd) = parser::parse_voice_command(text) {
            let _ = self.cmd_tx.send((ip.to_string(), cmd));
            true
        } else {
            false
        }
    }

    fn start_audio_listener(&self) {
        thread::spawn(move || {
            let host = cpal::default_host();
            let device = match host.default_input_device() {
                Some(d) => d,
                None => {
                    eprintln!("[Voice] No se encontró un dispositivo de entrada de audio por defecto.");
                    return;
                }
            };

            println!("[Voice] Micrófono por defecto inicializado correctamente.");

            let config = match device.default_input_config() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[Voice] Error al obtener la configuración del micrófono: {e}");
                    return;
                }
            };

            let err_fn = |err| eprintln!("[Voice] Error en el stream de audio: {err}");
            let stream_config = config.config();

            // 🧠 AQUÍ CARGARÍAS TU MODELO VOSK LOCAL EN EL FUTURO:
            // let model = Model::new("/ruta/al/modelo/vosk-model-small-es").expect("Error cargando modelo Vosk");
            // let mut recognizer = Recognizer::new(&model, config.sample_rate() as f32).expect("Error creando recognizer");

            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        Self::process_audio_buffer_f32(data);
                    },
                    err_fn,
                    None,
                ),
                cpal::SampleFormat::I16 => device.build_input_stream(
                    stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        Self::process_audio_buffer_i16(data);
                    },
                    err_fn,
                    None,
                ),
                cpal::SampleFormat::U16 => device.build_input_stream(
                    stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        Self::process_audio_buffer_u16(data);
                    },
                    err_fn,
                    None,
                ),
                format => {
                    eprintln!("[Voice] Formato de audio no soportado: {format:?}");
                    return;
                }
            };

            match stream {
                Ok(s) => {
                    if s.play().is_ok() {
                        println!("[Voice] 🎧 Escucha y reconocimiento de voz activo en segundo plano.");
                        loop {
                            thread::sleep(std::time::Duration::from_secs(1));
                        }
                    } else {
                        eprintln!("[Voice] No se pudo reproducir el stream.");
                    }
                }
                Err(e) => {
                    eprintln!("[Voice] Error al construir el stream: {e}");
                }
            }
        });
    }

    fn process_audio_buffer_f32(data: &[f32]) {
        if data.is_empty() {
            return;
        }

        let sum: f32 = data.iter().map(|&x| x * x).sum();
        let rms = (sum / data.len() as f32).sqrt();

        // Umbral de compuerta de ruido para evitar procesar silencio
        if rms < 0.02 {
            return;
        }

        // Aquí es donde el recognizer procesará las muestras de audio:
        // recognizer.accept_waveform(data);
        // let text_result = recognizer.result();
        // println!("[STT Palabra Detectada] -> {}", text_result);

        println!("[Audio Activo] Detectando voz... Volumen: {:.4}", rms);
    }

    fn process_audio_buffer_i16(_data: &[i16]) {}
    fn process_audio_buffer_u16(_data: &[u16]) {}
}