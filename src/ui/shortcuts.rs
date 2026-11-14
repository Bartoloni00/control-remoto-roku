use crate::roku::{Channel, Cmd};
use eframe::egui::{self, Color32, RichText, Stroke};

pub fn render(
    ui: &mut egui::Ui,
    button_color: Color32,
    send_cmd: impl Fn(Cmd),
    launch_ch: impl Fn(Channel),
) {
    let items: [(&str, Option<Channel>, Color32); 5] = [
        ("🏠 Home", None, Color32::from_rgb(50, 70, 110)),
        ("Netflix", Some(Channel::Netflix), Color32::from_rgb(229, 9, 20)),
        ("HBO / Max", Some(Channel::Hbo), Color32::from_rgb(88, 34, 180)),
        ("YouTube", Some(Channel::YouTube), Color32::from_rgb(255, 0, 0)),
        ("Spotify", Some(Channel::Spotify), Color32::from_rgb(29, 185, 84)),
    ];

    let total_slots = 5.0;
    let w = (ui.available_width() - 4.0 * 6.0) / total_slots;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for (label, ch, brand_color) in items {
            let btn = egui::Button::new(
                RichText::new(label)
                    .size(12.0)
                    .strong()
                    .color(Color32::WHITE),
            )
            .rounding(12.0)
            .fill(button_color)
            .stroke(Stroke::new(1.0_f32, brand_color.linear_multiply(0.6)));

            let resp = ui.add_sized([w, 44.0], btn);

            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            
            if resp.clicked() {
                match ch {
                    None => send_cmd(Cmd::Press("Home")),
                    Some(c) => launch_ch(c),
                }
            }
        }
    });
}