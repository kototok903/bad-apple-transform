#![allow(dead_code)] // for unused colors

#[derive(Clone, Copy)]
pub struct Color(pub [u8; 3]);

impl Color {
    pub const fn from_hex(hex: u32) -> Self {
        Self([
            ((hex >> 16) & 0xFF) as u8,
            ((hex >> 8) & 0xFF) as u8,
            (hex & 0xFF) as u8,
        ])
    }

    pub fn bytes(&self) -> [u8; 3] {
        self.0
    }

    pub fn blend(c1: Self, c2: Self, c2f: f64) -> Self {
        let c2f = c2f.clamp(0.0, 1.0);
        let blend_channel =
            |v1: u8, v2: u8| ((v1 as f64) + ((v2 as f64) - (v1 as f64)) * c2f).round() as u8;
        Self(std::array::from_fn(|i| blend_channel(c1.0[i], c2.0[i])))
    }
}

pub mod colors {
    use super::Color;

    pub const BLACK: Color = Color::from_hex(0x000000);
    pub const WHITE: Color = Color::from_hex(0xFFFFFF);
    pub const TEAL: Color = Color::from_hex(0x069494);
    pub const ORANGE: Color = Color::from_hex(0xFC6A03);
    pub const PURPLE: Color = Color::from_hex(0x6634B8);
    pub const PINK: Color = Color::from_hex(0xFFC0CB);
    pub const RED: Color = Color::from_hex(0xFF0000);
    pub const ORANGE2: Color = Color::from_hex(0xFF7F00);
    pub const YELLOW: Color = Color::from_hex(0xFFFF00);
    pub const GREEN: Color = Color::from_hex(0x00FF00);
    pub const CYAN: Color = Color::from_hex(0x00FFFF);
    pub const BLUE: Color = Color::from_hex(0x0000FF);
    pub const INDIGO: Color = Color::from_hex(0x4B0082);
    pub const VIOLET: Color = Color::from_hex(0x9400D3);
}
