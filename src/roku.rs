//! Cliente del External Control Protocol (ECP) de Roku.
//!
//! Las llamadas síncronas sirven para consultar estado. [`spawn`] las mueve a
//! otro hilo para que un click no congele la ventana.

use std::net::Ipv4Addr;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use ureq::Agent;

pub const DEFAULT_IP: &str = "192.168.0.38";
pub const PORT: u16 = 8060;

const NETFLIX_ID: &str = "12";
const YOUTUBE_ID: &str = "837";
const HBO_ID: &str = "61322";
const SPOTIFY_ID: &str = "22297";

const TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Select,
    Home,
}

impl Key {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Select => "Select",
            Self::Home => "Home",
        }
    }
}

/// Accesos de la fila inferior. El ID real sale de `query/apps` si el canal
/// está instalado; si no, se usa el de respaldo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    Netflix,
    YouTube,
    Hbo,
    Spotify,
}

impl Shortcut {
    pub fn label(self) -> &'static str {
        match self {
            Self::Netflix => "Netflix",
            Self::YouTube => "YouTube",
            Self::Hbo => "HBO",
            Self::Spotify => "Spotify",
        }
    }

    pub fn fallback_id(self) -> &'static str {
        match self {
            Self::Netflix => NETFLIX_ID,
            Self::YouTube => YOUTUBE_ID,
            Self::Hbo => HBO_ID,
            Self::Spotify => SPOTIFY_ID,
        }
    }

    pub const ALL: [Self; 4] = [Self::Netflix, Self::YouTube, Self::Hbo, Self::Spotify];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct App {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channels {
    pub netflix: String,
    pub youtube: String,
    pub hbo: String,
    pub spotify: String,
}

impl Channels {
    pub fn fallback() -> Self {
        Self {
            netflix: NETFLIX_ID.to_string(),
            youtube: YOUTUBE_ID.to_string(),
            hbo: HBO_ID.to_string(),
            spotify: SPOTIFY_ID.to_string(),
        }
    }

    pub fn resolve(apps: &[App]) -> Self {
        Self {
            netflix: resolve_id(apps, Shortcut::Netflix),
            youtube: resolve_id(apps, Shortcut::YouTube),
            hbo: resolve_id(apps, Shortcut::Hbo),
            spotify: resolve_id(apps, Shortcut::Spotify),
        }
    }

    pub fn id(&self, shortcut: Shortcut) -> &str {
        match shortcut {
            Shortcut::Netflix => &self.netflix,
            Shortcut::YouTube => &self.youtube,
            Shortcut::Hbo => &self.hbo,
            Shortcut::Spotify => &self.spotify,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceInfo {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub name: String,
    pub channels: Channels,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    KeyDown(Key),
    KeyUp(Key),
    KeyPress(Key),
    Launch(String),
    DeviceInfo,
    Apps,
    Status,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Response {
    Done,
    Device(DeviceInfo),
    Apps(Vec<App>),
    Status(Status),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    BadIp,
    Unreachable,
    MobileControlDisabled,
    BadResponse,
    Http(u16),
    InvalidAppId,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadIp => write!(f, "IP incorrecta"),
            Self::Unreachable => {
                write!(f, "Roku apagado, IP incorrecta o sin respuesta")
            }
            Self::MobileControlDisabled => write!(
                f,
                "“Control by mobile apps” desactivado (Ajustes > Sistema > Ajustes avanzados)"
            ),
            Self::BadResponse => write!(f, "Respuesta inesperada del Roku"),
            Self::Http(code) => write!(f, "El Roku respondió HTTP {code}"),
            Self::InvalidAppId => write!(f, "ID de canal inválido"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub struct Client {
    base: String,
    agent: Agent,
}

impl Client {
    pub fn new(ip: &str) -> Result<Self, Error> {
        let ip: Ipv4Addr = ip.trim().parse().map_err(|_| Error::BadIp)?;
        let config = Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .build();
        Ok(Self {
            base: format!("http://{ip}:{PORT}"),
            agent: Agent::new_with_config(config),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base
    }

    pub fn key_down(&self, key: Key) -> Result<(), Error> {
        self.post(&format!("/keydown/{}", key.as_str()))
    }

    pub fn key_up(&self, key: Key) -> Result<(), Error> {
        self.post(&format!("/keyup/{}", key.as_str()))
    }

    pub fn key_press(&self, key: Key) -> Result<(), Error> {
        self.post(&format!("/keypress/{}", key.as_str()))
    }

    pub fn launch(&self, app_id: &str) -> Result<(), Error> {
        if !valid_app_id(app_id) {
            return Err(Error::InvalidAppId);
        }
        self.post(&format!("/launch/{app_id}"))
    }

    pub fn device_info(&self) -> Result<DeviceInfo, Error> {
        let xml = self.get_text("/query/device-info")?;
        Ok(DeviceInfo {
            name: parse_device_name(&xml)?,
        })
    }

    pub fn query_apps(&self) -> Result<Vec<App>, Error> {
        let xml = self.get_text("/query/apps")?;
        parse_apps(&xml)
    }

    /// Nombre del equipo e IDs de los accesos, ya resueltos contra los canales instalados.
    pub fn status(&self) -> Result<Status, Error> {
        let info = self.device_info()?;
        let apps = self.query_apps()?;
        Ok(Status {
            name: info.name,
            channels: Channels::resolve(&apps),
        })
    }

    fn post(&self, path: &str) -> Result<(), Error> {
        let url = self.url(path);
        let mut response = self.agent.post(&url).send_empty().map_err(map_ureq)?;
        let _ = response.body_mut().read_to_string();
        Ok(())
    }

    fn get_text(&self, path: &str) -> Result<String, Error> {
        let url = self.url(path);
        let mut response = self.agent.get(&url).call().map_err(map_ureq)?;
        response
            .body_mut()
            .read_to_string()
            .map_err(|_| Error::BadResponse)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }
}

pub fn execute(ip: &str, request: Request) -> Result<Response, Error> {
    let client = Client::new(ip)?;
    match request {
        Request::KeyDown(key) => client.key_down(key).map(|()| Response::Done),
        Request::KeyUp(key) => client.key_up(key).map(|()| Response::Done),
        Request::KeyPress(key) => client.key_press(key).map(|()| Response::Done),
        Request::Launch(id) => client.launch(&id).map(|()| Response::Done),
        Request::DeviceInfo => client.device_info().map(Response::Device),
        Request::Apps => client.query_apps().map(Response::Apps),
        Request::Status => client.status().map(Response::Status),
    }
}

/// Lanza la petición en un hilo. `reply` recibe el resultado si la interfaz quiere
/// mostrar el nombre del equipo o el error; un click puede omitirlo.
pub fn spawn(
    ip: impl Into<String>,
    request: Request,
    reply: Option<mpsc::Sender<Result<Response, Error>>>,
) {
    let ip = ip.into();
    thread::spawn(move || {
        let result = execute(&ip, request);
        if let Some(reply) = reply {
            let _ = reply.send(result);
        }
    });
}

pub fn parse_device_name(xml: &str) -> Result<String, Error> {
    if !xml.contains("<device-info") {
        return Err(Error::BadResponse);
    }
    for tag in [
        "user-device-name",
        "friendly-device-name",
        "friendly-model-name",
        "model-name",
    ] {
        if let Some(name) = xml_text(xml, tag) {
            return Ok(name);
        }
    }
    Err(Error::BadResponse)
}

pub fn parse_apps(xml: &str) -> Result<Vec<App>, Error> {
    if !xml.contains("<apps") && !xml.contains("<app ") && !xml.contains("<app>") {
        return Err(Error::BadResponse);
    }

    let mut rest = xml;
    let mut apps = Vec::new();
    while let Some(start) = find_app_tag(rest) {
        rest = &rest[start..];
        let Some(tag_end) = rest.find('>') else {
            return Err(Error::BadResponse);
        };
        let open = &rest[..=tag_end];
        let id = attr(open, "id").unwrap_or("").trim();
        let self_closing = open.ends_with("/>");
        rest = &rest[tag_end + 1..];

        let name = if self_closing {
            String::new()
        } else {
            let Some(close) = rest.find("</app>") else {
                return Err(Error::BadResponse);
            };
            let name = decode_xml(rest[..close].trim());
            rest = &rest[close + "</app>".len()..];
            name
        };

        if valid_app_id(id) {
            apps.push(App {
                id: id.to_string(),
                name,
            });
        }
    }
    Ok(apps)
}

fn valid_app_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 12 && id.bytes().all(|byte| byte.is_ascii_digit())
}

fn resolve_id(apps: &[App], shortcut: Shortcut) -> String {
    let name_of = |app: &App| app.name.trim().to_ascii_lowercase();
    if let Some(app) = apps.iter().find(|app| exact_match(shortcut, &name_of(app))) {
        return app.id.clone();
    }
    if let Some(app) = apps.iter().find(|app| loose_match(shortcut, &name_of(app))) {
        return app.id.clone();
    }
    shortcut.fallback_id().to_string()
}

fn exact_match(shortcut: Shortcut, name: &str) -> bool {
    match shortcut {
        Shortcut::Netflix => name == "netflix",
        Shortcut::YouTube => name == "youtube",
        Shortcut::Hbo => matches!(name, "hbo" | "max" | "hbo max"),
        Shortcut::Spotify => name == "spotify",
    }
}

fn loose_match(shortcut: Shortcut, name: &str) -> bool {
    match shortcut {
        Shortcut::Netflix => name.starts_with("netflix"),
        Shortcut::YouTube => name.starts_with("youtube"),
        Shortcut::Hbo => name.contains("hbo"),
        Shortcut::Spotify => name.contains("spotify"),
    }
}

fn xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = start + xml[start..].find(&close)?;
    let text = decode_xml(xml[start..end].trim());
    if text.is_empty() { None } else { Some(text) }
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    for quote in ['"', '\''] {
        let pattern = format!("{name}={quote}");
        if let Some(index) = tag.find(&pattern) {
            let value = &tag[index + pattern.len()..];
            let end = value.find(quote)?;
            return Some(&value[..end]);
        }
    }
    None
}

fn find_app_tag(xml: &str) -> Option<usize> {
    let mut offset = 0;
    while let Some(rel) = xml[offset..].find("<app") {
        let abs = offset + rel;
        let next = xml[abs + 4..].chars().next();
        if matches!(next, Some(' ' | '\n' | '\r' | '\t' | '>' | '/')) {
            return Some(abs);
        }
        offset = abs + 4;
    }
    None
}

fn decode_xml(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        let Some(end) = rest.find(';') else {
            out.push_str(rest);
            return out;
        };
        let entity = &rest[1..end];
        if let Some(ch) = decode_entity(entity) {
            out.push(ch);
        } else {
            out.push('&');
            out.push_str(entity);
            out.push(';');
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => {
            let code = if let Some(hex) = entity.strip_prefix("#x") {
                u32::from_str_radix(hex, 16).ok()?
            } else if let Some(dec) = entity.strip_prefix('#') {
                dec.parse().ok()?
            } else {
                return None;
            };
            char::from_u32(code)
        }
    }
}

fn map_ureq(err: ureq::Error) -> Error {
    match err {
        ureq::Error::StatusCode(401 | 403) => Error::MobileControlDisabled,
        ureq::Error::StatusCode(code) => Error::Http(code),
        ureq::Error::BadUri(_) => Error::BadIp,
        ureq::Error::Timeout(_)
        | ureq::Error::HostNotFound
        | ureq::Error::ConnectionFailed
        | ureq::Error::Io(_)
        | ureq::Error::ConnectProxyFailed(_) => Error::Unreachable,
        _ => Error::Unreachable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEVICE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<device-info>
  <friendly-model-name>Roku Ultra</friendly-model-name>
  <friendly-device-name>Roku Ultra</friendly-device-name>
  <user-device-name>Sala &amp; TV</user-device-name>
</device-info>"#;

    const APPS_XML: &str = r#"<apps>
  <app id="12" type="appl" version="1.0">Netflix</app>
  <app id="837" subtype="ndka" type="appl" version="2.0">YouTube</app>
  <app id="55555" type="appl" version="1.0">Max</app>
  <app id="19977" type="appl" version="1.0">Spotify Music</app>
  <app id="99" type="appl" version="1.0">YouTube TV</app>
</apps>"#;

    #[test]
    fn builds_base_url_on_port_8060() {
        let client = Client::new(" 192.168.0.38 ").unwrap();
        assert_eq!(client.base_url(), "http://192.168.0.38:8060");
        assert_eq!(
            client.url("/keydown/Up"),
            "http://192.168.0.38:8060/keydown/Up"
        );
    }

    #[test]
    fn rejects_a_bad_ip_without_calling_the_network() {
        assert_eq!(Client::new("").unwrap_err(), Error::BadIp);
        assert_eq!(Client::new("192.168.0.38:8060").unwrap_err(), Error::BadIp);
        assert_eq!(
            Client::new("http://192.168.0.38").unwrap_err(),
            Error::BadIp
        );
        assert_eq!(
            execute("no-ip", Request::KeyPress(Key::Select)).unwrap_err(),
            Error::BadIp
        );
    }

    #[test]
    fn key_names_match_the_ecp_literals() {
        assert_eq!(Key::Up.as_str(), "Up");
        assert_eq!(Key::Down.as_str(), "Down");
        assert_eq!(Key::Left.as_str(), "Left");
        assert_eq!(Key::Right.as_str(), "Right");
        assert_eq!(Key::Select.as_str(), "Select");
        assert_eq!(Key::Home.as_str(), "Home");
    }

    #[test]
    fn launch_rejects_ids_that_are_not_digits() {
        let client = Client::new(DEFAULT_IP).unwrap();
        assert_eq!(client.launch("../query").unwrap_err(), Error::InvalidAppId);
        assert_eq!(client.launch("").unwrap_err(), Error::InvalidAppId);
    }

    #[test]
    fn device_name_prefers_the_user_name() {
        assert_eq!(parse_device_name(DEVICE_XML).unwrap(), "Sala & TV");
    }

    #[test]
    fn device_name_falls_back_when_the_user_name_is_empty() {
        let xml = r#"<device-info><user-device-name> </user-device-name><friendly-device-name>Living Room</friendly-device-name></device-info>"#;
        assert_eq!(parse_device_name(xml).unwrap(), "Living Room");
    }

    #[test]
    fn installed_names_override_fallback_ids() {
        let apps = parse_apps(APPS_XML).unwrap();
        let channels = Channels::resolve(&apps);
        assert_eq!(channels.netflix, "12");
        assert_eq!(channels.youtube, "837");
        assert_eq!(channels.hbo, "55555");
        assert_eq!(channels.spotify, "19977");
        assert_eq!(channels.id(Shortcut::Hbo), "55555");
    }

    #[test]
    fn missing_channels_keep_the_script_ids() {
        let channels = Channels::resolve(&[]);
        assert_eq!(channels, Channels::fallback());
        assert_eq!(channels.netflix, "12");
        assert_eq!(channels.youtube, "837");
        assert_eq!(channels.hbo, "61322");
        assert_eq!(channels.spotify, "22297");
    }

    #[test]
    fn hbo_max_matches_before_the_fallback() {
        let apps = parse_apps(r#"<apps><app id="61322" type="appl">HBO Max</app></apps>"#).unwrap();
        assert_eq!(Channels::resolve(&apps).hbo, "61322");
    }

    #[test]
    fn unexpected_xml_is_an_error() {
        assert_eq!(
            parse_device_name("<html></html>").unwrap_err(),
            Error::BadResponse
        );
        assert_eq!(parse_apps("not xml").unwrap_err(), Error::BadResponse);
    }

    #[test]
    fn every_request_checks_the_ip_before_using_the_network() {
        let requests = [
            Request::KeyDown(Key::Left),
            Request::KeyUp(Key::Right),
            Request::KeyPress(Key::Home),
            Request::Launch(Shortcut::Spotify.fallback_id().to_string()),
            Request::DeviceInfo,
            Request::Apps,
            Request::Status,
        ];
        for request in requests {
            assert_eq!(execute("no", request).unwrap_err(), Error::BadIp);
        }
        assert_eq!(
            Shortcut::ALL.map(Shortcut::label),
            ["Netflix", "YouTube", "HBO", "Spotify"]
        );
    }

    #[test]
    fn spawn_reports_a_bad_ip_on_the_channel() {
        let (tx, rx) = mpsc::channel();
        spawn("ip-mala", Request::DeviceInfo, Some(tx));
        let err = rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap_err();
        assert_eq!(err, Error::BadIp);
    }
}
