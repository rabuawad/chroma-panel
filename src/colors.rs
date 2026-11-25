use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

pub const PRESETS: [u32; 16] = [
    0xed464f, 0xf18028, 0xf8cd41, 0x9de14c, 0x41ef4a, 0x00aa56, 0x13b69e, 0x1bdce3, 0x4288f6,
    0x4242ed, 0x8145ed, 0xa855f7, 0xd333ed, 0xdd38aa, 0x922c62, 0x925936,
];

#[derive(Clone, Copy)]
pub struct Color {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}
impl Color {
    pub fn from_rgb(hex: u32) -> Self {
        let [r, g, b] = channels(hex).map(|n| n as f32 / 255.);
        let v = r.max(g).max(b);
        let d = v - r.min(g).min(b);
        let h = if d == 0. {
            0.
        } else if v == r {
            ((g - b) / d).rem_euclid(6.)
        } else if v == g {
            (b - r) / d + 2.
        } else {
            (r - g) / d + 4.
        };
        Self {
            h: h / 6.,
            s: if v == 0. { 0. } else { d / v },
            v,
        }
    }
    pub fn hex(self) -> u32 {
        let h = self.h.rem_euclid(1.) * 6.;
        let c = self.v * self.s;
        let x = c * (1. - (h.rem_euclid(2.) - 1.).abs());
        let rgb = match h as usize {
            0 => [c, x, 0.],
            1 => [x, c, 0.],
            2 => [0., c, x],
            3 => [0., x, c],
            4 => [x, 0., c],
            _ => [c, 0., x],
        };
        rgb.into_iter().fold(0, |hex, n| {
            (hex << 8) | ((n + self.v - c) * 255.).round().clamp(0., 255.) as u32
        })
    }
    pub fn hsl(self) -> [u32; 3] {
        let l = self.v * (1. - self.s / 2.);
        let s = if l <= 0. || l >= 1. {
            0.
        } else {
            (self.v - l) / l.min(1. - l)
        };
        [
            (self.h * 360.).round() as u32,
            (s * 100.).round() as u32,
            (l * 100.).round() as u32,
        ]
    }
}
pub fn channels(hex: u32) -> [u8; 3] {
    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8]
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub selected: u32,
    pub recent: Vec<u32>,
    pub favorites: [Option<u32>; 5],
    pub custom: Vec<u32>,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            selected: 0xa855f7,
            recent: vec![],
            favorites: [None; 5],
            custom: vec![],
        }
    }
}
impl Library {
    fn path() -> io::Result<PathBuf> {
        let root = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|s| PathBuf::from(s).join(".config")))
            .ok_or_else(|| io::Error::other("No configuration directory"))?;
        Ok(root.join("chroma-panel/colors.json"))
    }
    pub fn load() -> Self {
        Self::path()
            .ok()
            .and_then(|p| fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn remember(&mut self, hex: u32) {
        self.recent.retain(|&c| c != hex);
        self.recent.insert(0, hex);
        self.recent.truncate(8);
    }
    pub fn save(&self) -> io::Result<()> {
        let path = Self::path()?;
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir)?;
        let mut file = tempfile::NamedTempFile::new_in(dir)?;
        serde_json::to_writer(&mut file, self)?;
        file.persist(path)?;
        Ok(())
    }
}
