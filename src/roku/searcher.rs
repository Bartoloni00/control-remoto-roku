/// Define los comandos específicos para la entrada de texto y búsqueda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchCmd {
    Literal(char),
    Backspace,
}

impl SearchCmd {
    /// Devuelve la ruta HTTP correspondiente para el comando ECP de Roku.
    pub fn to_path(&self) -> String {
        match self {
            SearchCmd::Literal(c) => {
                // Roku requiere que los espacios se envíen codificados como %20
                if *c == ' ' {
                    "/keypress/Lit_%20".to_string()
                } else {
                    format!("/keypress/Lit_{c}")
                }
            }
            SearchCmd::Backspace => "/keypress/Backspace".to_string(),
        }
    }
}