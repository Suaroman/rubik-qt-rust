//! The one light. These two functions are the whole lighting model, and both
//! the cube renderer and the lighting panel call them, so the panel cannot drift
//! from what is actually being drawn.

use super::math::*;

pub const AMBIENT: f32 = 0.30;
pub const DIFFUSE: f32 = 0.70;

/// z strongly dominant so a camera-facing sticker lands at 0.97 brightness
/// (0.30 + 0.70 * 0.955). A light with y >= 0.5 cannot: every degree of viewing
/// pitch subtracts ly * sin(pitch) from the dot product of the face the viewer
/// looks straight at, which is what makes white read gray and yellow mustard.
/// y stays small but non-zero so top and bottom differ from the side faces.
pub fn light_base() -> Vec3 {
    normalize(&[0.26, 0.14, 0.955])
}

pub const ORBIT_RATE: f32 = 0.34; // rad/s of azimuth

pub struct Light {
    /// The live vector the shader reads: FROM the surface TOWARD the source.
    /// Directional — a direction, no position.
    pub dir: Vec3,
    pub base: Vec3,
    pub orbit: bool,
    pub phase: f32,
    /// The base vector's own elevation and azimuth, which the orbit preserves.
    pub elev: f32,
    pub elev_cos: f32,
    pub az0: f32,
}

impl Light {
    pub fn new() -> Light {
        let base = light_base();
        // Engaging the orbit precesses the base vector about world +y at its own
        // elevation, so phase 0 lands on the shipped vector and switching the
        // orbit on cannot snap the shading.
        let elev = base[1].asin();
        Light {
            dir: base,
            base,
            orbit: false,
            phase: 0.0,
            elev,
            elev_cos: elev.cos(),
            az0: base[0].atan2(base[2]),
        }
    }

    /// The phase integrates dt, so it is refresh-rate independent and continuous
    /// across every toggle.
    pub fn update(&mut self, dt: f32) {
        if self.orbit {
            self.phase = (self.phase + dt * ORBIT_RATE) % TAU;
            let az = self.az0 + self.phase;
            self.dir = [
                self.elev_cos * az.sin(),
                self.elev.sin(),
                self.elev_cos * az.cos(),
            ];
        } else {
            self.dir = self.base;
        }
    }

    #[inline]
    pub fn lambert(&self, n: &Vec3) -> f32 {
        let d = n[0] * self.dir[0] + n[1] * self.dir[1] + n[2] * self.dir[2];
        if d > 0.0 {
            d
        } else {
            0.0
        }
    }

    #[inline]
    pub fn brightness(&self, n: &Vec3) -> f32 {
        AMBIENT + DIFFUSE * self.lambert(n)
    }
}
