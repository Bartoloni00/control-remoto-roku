use crate::roku::{self, Channel, Cmd, Dir, Event};
use crate::{save_config, Config};
use eframe::egui::{
    self,
    epaint::{Mesh, Vertex, WHITE_UV},
    Align2, Color32, FontId, Key, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2,
};
use std::f32::consts::TAU;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

const BG: Color32 = Color32::from_rgb(14, 22, 48);
const BUTTON: Color32 = Color32::from_rgb(34, 50, 96);
const BUTTON_HOVER: Color32 = Color32::from_rgb(48, 70, 124);
const BUTTON_DOWN: Color32 = Color32::from_rgb(64, 88, 150);
const MUTED: Color32 = Color32::from_rgb(150, 165, 200);
const OK_GREEN: Color32 = Color32::from_rgb(120, 220, 140);
const ERR_RED: Color32 = Color32::from_rgb(255, 130, 120);

// Paradas del degradado: naranja → violeta → azul
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
}

impl RemoteApp {
    pub fn new(cc: &eframe::CreationContext<'_>, cfg: Config) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let (ev_tx, ev_rx) = channel();
        let cmd_tx = roku::spawn_worker(ev_tx.clone());
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
        };
        app.connect();
        app
    }

    /// Guarda la IP y consulta el nombre del equipo y sus apps.
    fn connect(&mut self) {
        self.ip = self.ip.trim().to_string();
        let _ = save_config(&Config { ip: self.ip.clone() });
        self.connected = false;
        self.status = "Conectando…".into();
        roku::spawn_query(self.ip.clone(), self.ev_tx.clone());
    }

    /// Envía un comando al hilo worker (fire-and-forget).
    fn send(&self, cmd: Cmd) {
        let _ = self.cmd_tx.send((self.ip.clone(), cmd));
    }

    fn launch(&self, ch: Channel) {
        self.send(Cmd::Launch(self.app_ids[ch as usize].clone()));
    }

    /// Procesa los mensajes que llegan del hilo de red.
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

    /// Teclado: flechas (mantener = keydown/keyup), Enter = OK, Escape/H = Home.
    fn read_keyboard(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            self.key_down = [false; 4];
            return;
        }
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
        self.key_down = down;
        if enter {
            self.send(Cmd::Press("Select"));
        }
        if home {
            self.send(Cmd::Press("Home"));
        }
    }

    /// Envía keydown/keyup solo cuando cambia el estado (ratón o teclado).
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

    /// Fila superior: IP editable, puerto fijo y estado/error.
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("IP:").strong());
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.ip)
                    .desired_width(140.0)
                    .hint_text(roku::DEFAULT_IP),
            );
            ui.label(RichText::new(":8060").color(MUTED));
            if ui.button("Conectar").clicked() || (resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))) {
                self.connect();
            }
        });
        ui.add_space(6.0);
        let color = if self.connected { OK_GREEN } else { ERR_RED };
        ui.label(RichText::new(&self.status).color(color).size(13.0));
    }

    /// Cruz de flechas, trébol de fondo y botón OK, todo dibujado con el painter.
    fn dpad(&mut self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(340.0), Sense::hover());
        let c = rect.center();
        let painter = ui.painter().clone();

        let trefoil_bounds = Rect::from_center_size(c, Vec2::splat(300.0));
        trefoil(&painter, c, 150.0, trefoil_bounds);

        let btn = 84.0;
        let off = 96.0;
        let centers = [
            c + Vec2::new(0.0, -off), // Up
            c + Vec2::new(0.0, off),  // Down
            c + Vec2::new(-off, 0.0), // Left
            c + Vec2::new(off, 0.0),  // Right
        ];
        let dirs = [(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)];

        for (i, d) in Dir::ALL.iter().enumerate() {
            let r = Rect::from_center_size(centers[i], Vec2::splat(btn));
            let resp = ui.interact(r, ui.id().with(("dir", i)), Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let pressed = resp.is_pointer_button_down_on();
            self.mouse_down[i] = pressed;

            let fill = if pressed {
                BUTTON_DOWN
            } else if resp.hovered() {
                BUTTON_HOVER
            } else {
                BUTTON
            };
            painter.rect_filled(r, 20.0, fill);
            let (dx, dy) = dirs[i];
            let _ = d;
            triangle(&painter, r.center(), Vec2::new(dx, dy), 16.0);
        }

        // OK circular con degradado
        let ok_bounds = Rect::from_center_size(c, Vec2::splat(80.0));
        let ok_resp = ui.interact(ok_bounds, ui.id().with("ok"), Sense::click());

        if ok_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        gradient_circle(&painter, c, 40.0, ok_bounds);
        painter.text(
            c,
            Align2::CENTER_CENTER,
            "OK",
            FontId::proportional(20.0),
            Color32::WHITE,
        );
        if ok_resp.clicked() {
            self.send(Cmd::Press("Select"));
        }
    }

    /// Fila de accesos directos con logos reales desde la API de Roku o tarjetas estilizadas.
    fn shortcuts(&mut self, ui: &mut egui::Ui) {
        // Definimos el label, el canal y un color distintivo para el borde/hover o fondo sutil
        let items: [(&str, Option<Channel>, Color32); 5] = [
            ("🏠 Home", None, Color32::from_rgb(50, 70, 110)),
            ("Netflix", Some(Channel::Netflix), Color32::from_rgb(229, 9, 20)),      // Rojo Netflix
            ("HBO / Max", Some(Channel::Hbo), Color32::from_rgb(88, 34, 180)),      // Violeta Max
            ("YouTube", Some(Channel::YouTube), Color32::from_rgb(255, 0, 0)),        // Rojo YouTube
            ("Spotify", Some(Channel::Spotify), Color32::from_rgb(29, 185, 84)),      // Verde Spotify
        ];

        let total_slots = 5.0;
        let w = (ui.available_width() - 4.0 * 6.0) / total_slots;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for (label, ch, brand_color) in items {
                // Creamos un botón con el color base y un toque del color de marca en el texto o bordes
                let btn = egui::Button::new(
                    RichText::new(label)
                        .size(12.0)
                        .strong()
                        .color(Color32::WHITE),
                )
                .rounding(12.0)
                .fill(BUTTON)
                .stroke(Stroke::new(1.0, brand_color.linear_multiply(0.6))); // Borde sutil con el color de la marca

                let resp = ui.add_sized([w, 44.0], btn);

                // Si el mouse pasa por encima, cambiamos el cursor a la manita
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                
                if resp.clicked() {
                    match ch {
                        None => self.send(Cmd::Press("Home")),
                        Some(c) => self.launch(c),
                    }
                }
            }
        });
    }
}

impl eframe::App for RemoteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();
        self.read_keyboard(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(16.0))
            .show(ctx, |ui| {
                self.top_bar(ui);
                ui.add_space(8.0);
                ui.vertical_centered(|ui| self.dpad(ui));
                ui.add_space(12.0);
                self.shortcuts(ui);
            });

        self.sync_holds();
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

/// Triángulo blanco apuntando en la dirección `d`, centrado en `c`.
fn triangle(p: &Painter, c: Pos2, d: Vec2, s: f32) {
    let n = Vec2::new(-d.y, d.x);
    let tip = c + d * s;
    let base = c - d * s * 0.5;
    let pts = vec![tip, base + n * s * 0.8, base - n * s * 0.8];
    p.add(Shape::convex_polygon(pts, Color32::WHITE, Stroke::NONE));
}

/// Contorno de trébol con degradado, a lo largo de la curva.
fn trefoil(p: &Painter, c: Pos2, r: f32, bounds: Rect) {
    const N: usize = 180;
    let pts: Vec<Pos2> = (0..=N)
        .map(|i| {
            let t = i as f32 / N as f32 * TAU;
            let rad = r * (0.8 + 0.2 * (3.0 * t).cos());
            c + Vec2::new(rad * t.cos(), rad * t.sin())
        })
        .collect();
    for w in pts.windows(2) {
        let mid = Pos2::new((w[0].x + w[1].x) / 2.0, (w[0].y + w[1].y) / 2.0);
        p.line_segment([w[0], w[1]], Stroke::new(4.0_f32, gradient_at(mid, bounds)));
    }
}

/// Círculo relleno con degradado (malla con color por vértice).
fn gradient_circle(p: &Painter, c: Pos2, r: f32, bounds: Rect) {
    const M: usize = 64;
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: c,
        uv: WHITE_UV,
        color: gradient_at(c, bounds),
    });
    for i in 0..M {
        let t = i as f32 / M as f32 * TAU;
        let pos = c + Vec2::new(r * t.cos(), r * t.sin());
        mesh.vertices.push(Vertex {
            pos,
            uv: WHITE_UV,
            color: gradient_at(pos, bounds),
        });
    }
    for i in 0..M as u32 {
        let a = 1 + i;
        let b = 1 + (i + 1) % M as u32;
        mesh.indices.extend([0, a, b]);
    }
    p.add(Shape::mesh(mesh));
}

/// Color del degradado naranja → violeta → azul según la posición (diagonal).
fn gradient_at(pos: Pos2, b: Rect) -> Color32 {
    let tx = ((pos.x - b.left()) / b.width()).clamp(0.0, 1.0);
    let ty = ((pos.y - b.top()) / b.height()).clamp(0.0, 1.0);
    let t = (tx + ty) / 2.0;
    if t < 0.5 {
        lerp(ORANGE, VIOLET, t * 2.0)
    } else {
        lerp(VIOLET, BLUE, (t - 0.5) * 2.0)
    }
}

fn lerp(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}