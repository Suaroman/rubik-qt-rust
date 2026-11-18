//! 3x3 matrices as row-major flat arrays, matching the source renderer:
//! `[m00 m01 m02 / m10 m11 m12 / m20 m21 m22]`.
//!
//! World space is +x right, +y up, +z toward the viewer, camera at (0, 0, CAM_Z).

pub type Mat3 = [f32; 9];
pub type Vec3 = [f32; 3];

pub const IDENT: Mat3 = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
pub const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
pub const TAU: f32 = std::f32::consts::TAU;

pub fn mul3(a: &Mat3, b: &Mat3) -> Mat3 {
    [
        a[0] * b[0] + a[1] * b[3] + a[2] * b[6],
        a[0] * b[1] + a[1] * b[4] + a[2] * b[7],
        a[0] * b[2] + a[1] * b[5] + a[2] * b[8],
        a[3] * b[0] + a[4] * b[3] + a[5] * b[6],
        a[3] * b[1] + a[4] * b[4] + a[5] * b[7],
        a[3] * b[2] + a[4] * b[5] + a[5] * b[8],
        a[6] * b[0] + a[7] * b[3] + a[8] * b[6],
        a[6] * b[1] + a[7] * b[4] + a[8] * b[7],
        a[6] * b[2] + a[7] * b[5] + a[8] * b[8],
    ]
}

pub fn apply3(m: &Mat3, v: &Vec3) -> Vec3 {
    [
        m[0] * v[0] + m[1] * v[1] + m[2] * v[2],
        m[3] * v[0] + m[4] * v[1] + m[5] * v[2],
        m[6] * v[0] + m[7] * v[1] + m[8] * v[2],
    ]
}

/// Right-hand rotation about x (0), y (1) or z (2).
pub fn rot_axis(axis: usize, ang: f32) -> Mat3 {
    let (c, s) = (ang.cos(), ang.sin());
    match axis {
        0 => [1.0, 0.0, 0.0, 0.0, c, -s, 0.0, s, c],
        1 => [c, 0.0, s, 0.0, 1.0, 0.0, -s, 0.0, c],
        _ => [c, -s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0],
    }
}

pub fn cross(a: &Vec3, b: &Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn normalize(v: &Vec3) -> Vec3 {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let l = if l == 0.0 { 1.0 } else { l };
    [v[0] / l, v[1] / l, v[2] / l]
}

/// Rounds a rotation matrix back to exact integers so repeated slice commits
/// cannot accumulate float drift over the animation.
pub fn snap3(m: &Mat3) -> Mat3 {
    let mut o = [0.0f32; 9];
    for i in 0..9 {
        o[i] = m[i].round();
    }
    o
}

/// Axis/angle decomposition of a rotation matrix, for the inspector's readout.
pub fn axis_angle(m: &Mat3) -> (f32, Option<Vec3>) {
    let mut cs = (m[0] + m[4] + m[8] - 1.0) / 2.0;
    cs = cs.clamp(-1.0, 1.0);
    let ang = cs.acos();
    if ang < 1e-6 {
        return (0.0, None);
    }
    let ax = if (ang - std::f32::consts::PI).abs() < 1e-6 {
        let mut best = -1.0f32;
        let mut bi = 0usize;
        for i in 0..3 {
            let vx = m[i] + if i == 0 { 1.0 } else { 0.0 };
            let vy = m[3 + i] + if i == 1 { 1.0 } else { 0.0 };
            let vz = m[6 + i] + if i == 2 { 1.0 } else { 0.0 };
            let l = vx * vx + vy * vy + vz * vz;
            if l > best {
                best = l;
                bi = i;
            }
        }
        [
            m[bi] + if bi == 0 { 1.0 } else { 0.0 },
            m[3 + bi] + if bi == 1 { 1.0 } else { 0.0 },
            m[6 + bi] + if bi == 2 { 1.0 } else { 0.0 },
        ]
    } else {
        [m[7] - m[5], m[2] - m[6], m[3] - m[1]]
    };
    (ang * 180.0 / std::f32::consts::PI, Some(normalize(&ax)))
}

pub fn deg(rad: f32) -> f32 {
    rad * 180.0 / std::f32::consts::PI
}

pub fn clamp01(v: f32) -> f32 {
    if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

pub fn ease_in_out_cubic(k: f32) -> f32 {
    if k < 0.5 {
        4.0 * k * k * k
    } else {
        1.0 - (-2.0 * k + 2.0).powi(3) / 2.0
    }
}
