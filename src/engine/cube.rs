//! Cube state: 26 cubies (3x3x3 minus the hidden core), each carrying a grid
//! position and an orientation matrix. Stickers live in cubie-local space, so
//! colours are carried by the pieces and are never repainted from position.

use super::math::*;
use crate::color::Color;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Face {
    R,
    L,
    U,
    D,
    F,
    B,
}

/// `DIRS` order — also the order the lighting panel lists faces in.
pub const DIRS: [Face; 6] = [Face::R, Face::L, Face::U, Face::D, Face::F, Face::B];

impl Face {
    pub fn key(self) -> &'static str {
        match self {
            Face::R => "R",
            Face::L => "L",
            Face::U => "U",
            Face::D => "D",
            Face::F => "F",
            Face::B => "B",
        }
    }

    /// Base colour at full saturation; brightness is applied by shading only.
    pub fn color(self) -> Color {
        match self {
            Face::U => Color::rgb(245, 245, 245), // white  — top
            Face::D => Color::rgb(255, 204, 0),   // yellow — bottom
            Face::F => Color::rgb(0, 150, 70),    // green  — front (+z)
            Face::B => Color::rgb(0, 74, 200),    // blue   — back  (-z)
            Face::R => Color::rgb(204, 24, 24),   // red    — right (+x)
            Face::L => Color::rgb(255, 138, 0),   // orange — left  (-x)
        }
    }

    /// Outward normal.
    pub fn n(self) -> Vec3 {
        match self {
            Face::R => [1.0, 0.0, 0.0],
            Face::L => [-1.0, 0.0, 0.0],
            Face::U => [0.0, 1.0, 0.0],
            Face::D => [0.0, -1.0, 0.0],
            Face::F => [0.0, 0.0, 1.0],
            Face::B => [0.0, 0.0, -1.0],
        }
    }

    /// Axis this face's normal runs along, and which side of it.
    pub fn axis_at(self) -> (usize, i32) {
        match self {
            Face::R => (0, 1),
            Face::L => (0, -1),
            Face::U => (1, 1),
            Face::D => (1, -1),
            Face::F => (2, 1),
            Face::B => (2, -1),
        }
    }

    pub fn face_key_on_axis(axis: usize, side: i32) -> Option<Face> {
        DIRS.into_iter().find(|d| d.axis_at().0 == axis && d.axis_at().1 == side)
    }

    /// Nearest face to an arbitrary direction (post-rotation, snapped).
    pub fn of_normal(v: &Vec3) -> Option<Face> {
        let k = [v[0].round(), v[1].round(), v[2].round()];
        DIRS.into_iter().find(|d| d.n() == k)
    }
}

pub const BODY_COLOR: Color = Color::rgb(26, 26, 34); // near-black plastic frame & gaps

/// Six face directions. u/v span the face so corners run in perimeter order.
#[derive(Copy, Clone)]
pub struct FaceBasis {
    pub n: Vec3,
    pub u: Vec3,
    pub v: Vec3,
}

pub fn face_basis(n: &Vec3) -> FaceBasis {
    let refv = if n[1].abs() > 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let u = normalize(&cross(&refv, n));
    // u x v === n, so (u, v) is CCW seen from outside.
    let v = cross(n, &u);
    FaceBasis { n: *n, u, v }
}

#[derive(Copy, Clone)]
pub struct Sticker {
    pub n: Vec3,
    pub u: Vec3,
    pub v: Vec3,
    /// `None` for an interior face: still drawn, because a slice mid-turn exposes
    /// its cross-section to the camera.
    pub color: Option<Face>,
}

#[derive(Clone)]
pub struct Cubie {
    pub p: [i32; 3],
    pub home: [i32; 3],
    pub rot: Mat3,
    pub faces: [Sticker; 6],
}

pub fn build_cube() -> Vec<Cubie> {
    let mut cubies = Vec::with_capacity(26);
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                if x == 0 && y == 0 && z == 0 {
                    continue;
                }
                let p = [x, y, z];
                let mut faces = std::array::from_fn(|_| Sticker {
                    n: [0.0; 3],
                    u: [0.0; 3],
                    v: [0.0; 3],
                    color: None,
                });
                for (i, d) in DIRS.iter().enumerate() {
                    let basis = face_basis(&d.n());
                    let (axis, at) = d.axis_at();
                    faces[i] = Sticker {
                        n: basis.n,
                        u: basis.u,
                        v: basis.v,
                        color: if p[axis] == at { Some(*d) } else { None },
                    };
                }
                cubies.push(Cubie {
                    p,
                    home: p,
                    rot: IDENT,
                    faces,
                });
            }
        }
    }
    cubies
}

/// Clockwise as seen from outside the turned face. Right-hand rule: a clockwise
/// quarter turn about a face whose outward normal is +axis is -90 deg about that
/// axis.
#[derive(Copy, Clone, Debug)]
pub struct MoveSpec {
    pub axis: usize,
    pub layer: i32,
    pub sign: i32,
}

pub fn move_base(name: char) -> MoveSpec {
    match name {
        'R' => MoveSpec { axis: 0, layer: 1, sign: -1 },
        'L' => MoveSpec { axis: 0, layer: -1, sign: 1 },
        'U' => MoveSpec { axis: 1, layer: 1, sign: -1 },
        'D' => MoveSpec { axis: 1, layer: -1, sign: 1 },
        'F' => MoveSpec { axis: 2, layer: 1, sign: -1 },
        _ => MoveSpec { axis: 2, layer: -1, sign: 1 },
    }
}

pub fn parse_move(name: &str) -> MoveSpec {
    let base = move_base(name.chars().next().unwrap_or('R'));
    let sign = if name.contains('\'') { -base.sign } else { base.sign };
    MoveSpec { axis: base.axis, layer: base.layer, sign }
}

/// Commits an exact 90 deg turn of one layer of 9 cubies into the state.
pub fn apply_move(cubies: &mut [Cubie], name: &str) -> usize {
    let m = parse_move(name);
    let r = rot_axis(m.axis, m.sign as f32 * HALF_PI);
    let mut turned = 0;
    for c in cubies.iter_mut() {
        if c.p[m.axis] != m.layer {
            continue;
        }
        let p = apply3(&r, &(c.p.map(|v| v as f32)));
        c.p = [p[0].round() as i32, p[1].round() as i32, p[2].round() as i32];
        c.rot = snap3(&mul3(&r, &c.rot));
        turned += 1;
    }
    turned
}

pub fn is_solved(cubies: &[Cubie]) -> bool {
    cubies.iter().all(|c| {
        c.p == c.home && (0..9).all(|k| c.rot[k] == IDENT[k])
    })
}

/// 'R' -> "R'", "R'" -> 'R'.
pub fn inverse_of(name: &str) -> String {
    if name.contains('\'') {
        name.chars().next().unwrap().to_string()
    } else {
        format!("{}'", name)
    }
}

pub fn piece_class(home: &[i32; 3]) -> &'static str {
    let nz = home.iter().filter(|v| **v != 0).count();
    match nz {
        3 => "corner",
        2 => "edge",
        _ => "centre",
    }
}

pub fn oriented(m: &Mat3) -> bool {
    (0..9).all(|k| m[k] == IDENT[k])
}
