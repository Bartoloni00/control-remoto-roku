use crate::roku::{Cmd, SearchCmd};
use eframe::egui::{self, Key, RichText};

pub fn render(
    ui: &mut egui::Ui,
    search_text: &mut String,
    focus_search: &mut bool,
    send_cmd: impl Fn(Cmd),
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("⌨️").size(14.0));
        
        let text_edit = egui::TextEdit::singleline(search_text)
            .desired_width(ui.available_width() - 70.0)
            .hint_text("Ctrl+F para buscar (Escribe 1 a 1)...");

        let resp = ui.add(text_edit);

        if *focus_search {
            resp.request_focus();
            *focus_search = false;
        }

        if resp.changed() {
            if let Some(c) = search_text.chars().last() {
                send_cmd(Cmd::Text(SearchCmd::Literal(c)));
            }
        }

        if resp.has_focus() && ui.input(|i| i.key_pressed(Key::Backspace)) {
            if !search_text.is_empty() {
                search_text.pop();
            }
            send_cmd(Cmd::Text(SearchCmd::Backspace));
        }
    });
}