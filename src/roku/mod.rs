pub mod basic_keys;
pub mod shortcuts;
pub mod searcher;
pub use basic_keys::Dir;
pub use shortcuts::{Channel, InstalledApp, resolve_id};
use std::sync::mpsc::Sender;
use std::time::Duration;
pub use searcher::SearchCmd;

pub const PORT: u16 = 8060;
pub const DEFAULT_IP: &str = "192.168.0.38";
const TIMEOUT: Duration = Duration::from_secs(3);

/// Órdenes que se ejecutan en el hilo worker, en orden.
#[derive(Debug)]
pub enum Cmd {
    KeyDown(Dir),
    KeyUp(Dir),
    Press(&'static str),
    Launch(String),
    Text(searcher::SearchCmd), // Para el teclado virtual, si se pulsa un carácter o retroceso
}

/// Mensajes del hilo de red hacia la UI.
#[derive(Debug)]
pub enum Event {
    DeviceName(String),
    DeviceError(String),
    Apps(Vec<InstalledApp>),
    Error(String),
}

fn url(ip: &str, path: &str) -> String {
    format!("http://{ip}:{PORT}{path}")
}

fn describe_error(e: &ureq::Error) -> String {
    match e {
        ureq::Error::Status(403, _) => {
            "Control by mobile apps desactivado (Ajustes > Sistema > Ajustes avanzados)".into()
        }
        ureq::Error::Status(code, _) => format!("El Roku respondió HTTP {code}"),
        ureq::Error::Transport(_) => "Roku apagado o IP incorrecta".into(),
        #[allow(unreachable_patterns)]
        _ => "Error de red".into(),
    }
}

fn post(ip: &str, path: &str) -> Result<(), String> {
    ureq::post(&url(ip, path))
        .timeout(TIMEOUT)
        .call()
        .map(|_| ())
        .map_err(|e| describe_error(&e))
}

fn get_text(ip: &str, path: &str) -> Result<String, String> {
    ureq::get(&url(ip, path))
        .timeout(TIMEOUT)
        .call()
        .map_err(|e| describe_error(&e))?
        .into_string()
        .map_err(|e| e.to_string())
}

fn xml_tag(s: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = s.find(&open)? + open.len();
    let end = s[start..].find(&close)? + start;
    Some(s[start..end].trim().to_string())
}

fn xml_attr(attrs: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = attrs.find(&key)? + key.len();
    let end = attrs[start..].find('"')? + start;
    Some(attrs[start..end].to_string())
}

pub fn device_name(ip: &str) -> Result<String, String> {
    let body = get_text(ip, "/query/device-info")?;
    Ok(xml_tag(&body, "user-device-name")
        .or_else(|| xml_tag(&body, "friendly-device-name"))
        .unwrap_or_else(|| "Roku".into()))
}

pub fn installed_apps(ip: &str) -> Result<Vec<InstalledApp>, String> {
    let body = get_text(ip, "/query/apps")?;
    let mut apps = Vec::new();
    let mut rest = body.as_str();
    while let Some(pos) = rest.find("<app ") {
        rest = &rest[pos + 5..];
        let Some(tag_end) = rest.find('>') else { break };
        let id = xml_attr(&rest[..tag_end], "id");
        let after = &rest[tag_end + 1..];
        let Some(close) = after.find("</app>") else { break };
        let name = after[..close]
            .trim()
            .replace("&amp;", "&");
        if let Some(id) = id {
            apps.push(InstalledApp { id, name });
        }
        rest = &after[close + 6..];
    }
    Ok(apps)
}

pub fn run_cmd(ip: &str, cmd: &Cmd) -> Result<(), String> {
    match cmd {
        Cmd::KeyDown(d) => post(ip, &format!("/keydown/{}", d.key())),
        Cmd::KeyUp(d) => post(ip, &format!("/keyup/{}", d.key())),
        Cmd::Press(k) => post(ip, &format!("/keypress/{k}")),
        Cmd::Launch(id) => post(ip, &format!("/launch/{id}")),
        Cmd::Text(search_cmd) => post(ip, &search_cmd.to_path()),
    }
}

pub fn spawn_worker(events: Sender<Event>) -> Sender<(String, Cmd)> {
    let (tx, rx) = std::sync::mpsc::channel::<(String, Cmd)>();
    std::thread::spawn(move || {
        while let Ok((ip, cmd)) = rx.recv() {
            if let Err(e) = run_cmd(&ip, &cmd) {
                let _ = events.send(Event::Error(e));
            }
        }
    });
    tx
}

pub fn spawn_query(ip: String, events: Sender<Event>) {
    std::thread::spawn(move || match device_name(&ip) {
        Ok(name) => {
            let _ = events.send(Event::DeviceName(name));
            if let Ok(apps) = installed_apps(&ip) {
                let _ = events.send(Event::Apps(apps));
            }
        }
        Err(e) => {
            let _ = events.send(Event::DeviceError(e));
        }
    });
}