#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

    pub fn key(self) -> &'static str {
        match self {
            Dir::Up => "Up",
            Dir::Down => "Down",
            Dir::Left => "Left",
            Dir::Right => "Right",
        }
    }
}