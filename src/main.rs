mod roku;
mod ui;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub ip: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ip: roku::DEFAULT_IP.to_string(),
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("control-remoto-roku")
        .join("config.toml")
}

pub fn load_config() -> Config {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(cfg: &Config) -> std::io::Result<()> {
    let path = config_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = toml::to_string_pretty(cfg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, text)
}

fn main() -> eframe::Result<()> {
    let cfg = load_config();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([460.0, 680.0])
            .with_title("Control remoto Roku"),
        ..Default::default()
    };
    eframe::run_native(
        "Control remoto Roku",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            // Quitamos el "Ok(" de aquí, ya que la closure espera directamente el Box<dyn App>
            Box::new(ui::RemoteApp::new(cc, cfg))
        }),
    )
}
