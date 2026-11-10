#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Netflix,
    Hbo,
    YouTube,
    Spotify,
}

impl Channel {
    pub const ALL: [Channel; 4] = [
        Channel::Netflix,
        Channel::Hbo,
        Channel::YouTube,
        Channel::Spotify,
    ];

    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            Channel::Netflix => "Netflix",
            Channel::Hbo => "HBO",
            Channel::YouTube => "YouTube",
            Channel::Spotify => "Spotify",
        }
    }

    pub fn default_id(self) -> &'static str {
        match self {
            Channel::Netflix => "12",
            Channel::Hbo => "61322",
            Channel::YouTube => "837",
            Channel::Spotify => "22297",
        }
    }

    pub fn aliases(self) -> &'static [&'static str] {
        match self {
            Channel::Netflix => &["Netflix"],
            Channel::Hbo => &["HBO Max", "Max", "HBO"],
            Channel::YouTube => &["YouTube"],
            Channel::Spotify => &["Spotify"],
        }
    }
}

#[derive(Clone, Debug)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
}

pub fn resolve_id(ch: Channel, installed: &[InstalledApp]) -> String {
    installed
        .iter()
        .find(|a| ch.aliases().iter().any(|al| a.name.eq_ignore_ascii_case(al)))
        .map(|a| a.id.clone())
        .unwrap_or_else(|| ch.default_id().to_string())
}