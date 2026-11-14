use crate::roku::{self};
use eframe::egui::{self, Key, RichText, Color32};

pub fn render(
    ui: &mut egui::Ui,
    ip: &mut String,
    connected: bool,
    status: &str,
    ok_green: Color32,
    err_red: Color32,
    muted: Color32,
    on_connect: impl FnOnce(),
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("IP:").strong());
        let resp = ui.add(
            egui::TextEdit::singleline(ip)
                .desired_width(140.0)
                .hint_text(roku::DEFAULT_IP),
        );
        ui.label(RichText::new(":8060").color(muted));
        if ui.button("Conectar").clicked() || (resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))) {
            on_connect();
        }
    });
    ui.add_space(6.0);
    let color = if connected { ok_green } else { err_red };
    ui.label(RichText::new(status).color(color).size(13.0));
}