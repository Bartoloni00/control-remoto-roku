pub mod header;
pub mod searchbar;
pub mod dpad;
pub mod shortcuts;

use crate::roku::{self, Channel, Cmd, Dir, Event};
use crate::{save_config, Config};
use eframe::egui::{self, Color32, Key, RichText};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use crate::voice::VoiceManager;

const BG: Color32 = Color32::from_rgb(14, 22, 48);
const BUTTON: Color32 = Color32::from_rgb(34, 50, 96);
const BUTTON_HOVER: Color32 = Color32::from_rgb(48, 70, 124);
const BUTTON_DOWN: Color32 = Color32::from_rgb(64, 88, 150);
const MUTED: Color32 = Color32::from_rgb(150, 165, 200);
const OK_GREEN: Color32 = Color32::from_rgb(120, 220, 140);
const ERR_RED: Color32 = Color32::from_rgb(255, 130, 120);

const ORANGE: (u8, u8, u8) = (255, 150, 40);
const VIOLET: (u8, u8, u8) = (150, 70, 220);
const BLUE: (u8, u8, u8) = (40, 120, 255);

pub struct RemoteApp {
    ip: String,
    status: String,
    connected: bool,
    app_ids: [String; 4],
    held: [bool; 4],
    mouse_down: [bool; 4],
    key_down: [bool; 4],
    cmd_tx: Sender<(String, Cmd)>,
    ev_tx: Sender<Event>,
    ev_rx: Receiver<Event>,
    search_text: String,
    focus_search: bool,
    voice_manager: VoiceManager,
    voice_input: String, // Campo de texto temporal para probar comandos de voz por escrito o simulación
}

impl RemoteApp {

    pub fn new(cc: &eframe::CreationContext<'_>, cfg: Config) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let (ev_tx, ev_rx) = channel();
        let cmd_tx = roku::spawn_worker(ev_tx.clone());
        let voice_manager = VoiceManager::new(cmd_tx.clone());

        let mut app = Self {
            ip: cfg.ip,
            status: "Conectando…".into(),
            connected: false,
            app_ids: Channel::ALL.map(|c| c.default_id().to_string()),
            held: [false; 4],
            mouse_down: [false; 4],
            key_down: [false; 4],
            cmd_tx,
            ev_tx,
            ev_rx,
            search_text: String::new(),
            focus_search: false,
            voice_manager,
            voice_input: String::new(),
        };
        app.connect();
        app
    }

    fn connect(&mut self) {
        self.ip = self.ip.trim().to_string();
        let _ = save_config(&Config { ip: self.ip.clone() });
        self.connected = false;
        self.status = "Conectando…".into();
        roku::spawn_query(self.ip.clone(), self.ev_tx.clone());
    }

    fn send(&self, cmd: Cmd) {
        let _ = self.cmd_tx.send((self.ip.clone(), cmd));
    }

    fn launch(&self, ch: Channel) {
        self.send(Cmd::Launch(self.app_ids[ch as usize].clone()));
    }

    fn poll_events(&mut self) {
        while let Ok(ev) = self.ev_rx.try_recv() {
            match ev {
                Event::DeviceName(name) => {
                    self.connected = true;
                    self.status = format!("Conectado: {name}");
                }
                Event::DeviceError(e) | Event::Error(e) => {
                    self.connected = false;
                    self.status = e;
                }
                Event::Apps(list) => {
                    for ch in Channel::ALL {
                        self.app_ids[ch as usize] = roku::resolve_id(ch, &list);
                    }
                }
            }
        }
    }

    fn read_keyboard(&mut self, ctx: &egui::Context) {
        let (down, enter, home) = ctx.input(|i| {
            (
                [
                    i.key_down(Key::ArrowUp),
                    i.key_down(Key::ArrowDown),
                    i.key_down(Key::ArrowLeft),
                    i.key_down(Key::ArrowRight),
                ],
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::Escape) || i.key_pressed(Key::H),
            )
        });

        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::F)) {
            self.focus_search = true;
        }

        if ctx.wants_keyboard_input() {
            self.key_down = [false; 4];
            return;
        }

        self.key_down = down;
        if enter {
            self.send(Cmd::Press("Select"));
        }
        if home {
            self.send(Cmd::Press("Home"));
        }
    }

    fn sync_holds(&mut self) {
        for d in Dir::ALL {
            let i = d as usize;
            let want = self.mouse_down[i] || self.key_down[i];
            if want != self.held[i] {
                self.held[i] = want;
                self.send(if want { Cmd::KeyDown(d) } else { Cmd::KeyUp(d) });
            }
        }
    }
}

impl eframe::App for RemoteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();
        self.read_keyboard(ctx);

        // Extraemos los datos necesarios para evitar conflictos de múltiples mutaciones prestadas de `self`
        let ip = self.ip.clone();
        let cmd_tx = self.cmd_tx.clone();
        let connected = self.connected;
        let status = self.status.clone();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(16.0))
            .show(ctx, |ui| {
                // Componente Header
                let mut ip_changed = false;
                header::render(
                    ui,
                    &mut self.ip,
                    connected,
                    &status,
                    OK_GREEN,
                    ERR_RED,
                    MUTED,
                    || {
                        ip_changed = true; // Solo activamos la bandera
                    },
                );

                if ip_changed {
                    self.connect(); // Ejecutamos la conexión de manera segura aquí fuera
                }
                
                ui.add_space(6.0);

                // Componente SearchBar (clonamos ip y tx localmente para los closures)
                let ip_search = ip.clone();
                let tx_search = cmd_tx.clone();
                searchbar::render(
                    ui,
                    &mut self.search_text,
                    &mut self.focus_search,
                    move |cmd| {
                        let _ = tx_search.send((ip_search.clone(), cmd));
                    },
                );

                ui.add_space(8.0);

                // Componente D-pad
                let ip_dpad = ip.clone();
                let tx_dpad = cmd_tx.clone();
                ui.vertical_centered(|ui| {
                    dpad::render(
                        ui,
                        &mut self.mouse_down,
                        BUTTON,
                        BUTTON_HOVER,
                        BUTTON_DOWN,
                        ORANGE,
                        VIOLET,
                        BLUE,
                        move |cmd| {
                            let _ = tx_dpad.send((ip_dpad.clone(), cmd));
                        },
                    );
                });

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("🎙️ Voz/IA:").strong());
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.voice_input)
                            .desired_width(ui.available_width() - 80.0)
                            .hint_text("Ej: 'Abrir Netflix', 'Subir', 'Home'..."),
                    );
                    if ui.button("Enviar").clicked() || resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.voice_manager.handle_spoken_text(&self.ip, &self.voice_input);
                        self.voice_input.clear();
                    }
                });

                // Componente Shortcuts
                shortcuts::render(
                    ui,
                    BUTTON,
                    |cmd| self.send(cmd),
                    |ch| self.launch(ch),
                );
            });

        self.sync_holds();
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}