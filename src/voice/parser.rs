use crate::roku::{Cmd};

/// Analiza una cadena de texto (proveniente de voz o IA) y la mapea a un comando de Roku.
pub fn parse_voice_command(text: &str) -> Option<Cmd> {
    let clean = text.trim().to_lowercase();

    if clean.contains("arriba") {
        Some(Cmd::Press("Up"))
    } else if clean.contains("abajo") {
        Some(Cmd::Press("Down"))
    } else if clean.contains("izquierda") {
        Some(Cmd::Press("Left"))
    } else if clean.contains("derecha") {
        Some(Cmd::Press("Right"))
    } else if clean.contains("enter") || clean.contains("seleccionar") || clean.contains("ok") {
        Some(Cmd::Press("Select"))
    } else if clean.contains("home") || clean.contains("inicio") || clean.contains("casita") {
        Some(Cmd::Press("Home"))
    } else if clean.contains("netflix") {
        Some(Cmd::Launch("12".to_string()))
    } else if clean.contains("hbo") || clean.contains("max") {
        Some(Cmd::Launch("61322".to_string()))
    } else if clean.contains("youtube") {
        Some(Cmd::Launch("837".to_string()))
    } else if clean.contains("spotify") {
        Some(Cmd::Launch("22297".to_string()))
    } else {
        None
    }
}