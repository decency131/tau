#[derive(Clone, Copy)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const RED: Self = Self { r: 32, g: 0, b: 0 };
    pub const GREEN: Self = Self { r: 0, g: 32, b: 0 };
    pub const BLUE: Self = Self { r: 0, g: 0, b: 32 };
    pub const YELLOW: Self = Self { r: 32, g: 16, b: 0 };
    pub const CYAN: Self = Self { r: 0, g: 24, b: 24 };
    pub const MAGENTA: Self = Self { r: 24, g: 0, b: 24 };
    pub const WHITE: Self = Self {
        r: 24,
        g: 24,
        b: 24,
    };
}
