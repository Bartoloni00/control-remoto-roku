pub mod parser;

use crate::roku::Cmd;
use std::sync::mpsc::Sender;

pub struct VoiceManager {
    cmd_tx: Sender<(String, Cmd)>,
}

impl VoiceManager {
    pub fn new(cmd_tx: Sender<(String, Cmd)>) -> Self {
        Self { cmd_tx }
    }

    /// Procesa una orden de voz entrante y la ejecuta si es válida para la IP dada.
    pub fn handle_spoken_text(&self, ip: &str, text: &str) -> bool {
        if let Some(cmd) = parser::parse_voice_command(text) {
            let _ = self.cmd_tx.send((ip.to_string(), cmd));
            true
        } else {
            false
        }
    }
}