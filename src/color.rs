//! The sticker palette. The engine only needs straight 8-bit RGB; everything
//! else about colour (shading, gradients, glow) now belongs to Qt.

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    /// `#rrggbb`, the form QML colour properties take.
    pub fn css(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}
