//! Procedural geometry for the Qt Quick 3D view, generated once at start-up and
//! handed to QML's `ProceduralMesh`.
//!
//! The softbuffer renderer drew every cubie as six flat quads. Here each cubie is
//! a rounded box (the plastic) carrying bevelled, rounded-corner stickers, so
//! specular highlights run along the edges the way they do on a real cube.
//!
//! Units: one cubie is 100 scene units (Qt Quick 3D's own primitive size), so
//! the engine's 0.5 half-extents become 50.

use serde_json::{json, Value};

/// Half-extent of the plastic body. Slightly under 50 so a hairline gap
/// separates neighbours and the turning slice reads as a separate block.
pub const BODY_HALF: f64 = 48.6;
/// Edge radius of the plastic body.
pub const BODY_RADIUS: f64 = 7.0;
/// Sticker half-size: `STICKER_HALF / CUBIE_HALF` = 82% of the 50-unit face.
pub const STICKER_HALF: f64 = 41.0;
pub const STICKER_RADIUS: f64 = 6.5;
pub const STICKER_THICK: f64 = 1.3;
pub const STICKER_BEVEL: f64 = 0.9;

#[derive(Default)]
pub struct Mesh {
    pub pos: Vec<f64>,
    pub nrm: Vec<f64>,
    pub idx: Vec<u32>,
}

impl Mesh {
    fn vert(&mut self, p: [f64; 3], n: [f64; 3]) -> u32 {
        let i = (self.pos.len() / 3) as u32;
        self.pos.extend_from_slice(&p);
        self.nrm.extend_from_slice(&n);
        i
    }

    pub fn to_json(&self) -> Value {
        json!({ "pos": self.pos, "nrm": self.nrm, "idx": self.idx })
    }

    pub fn tri_count(&self) -> usize {
        self.idx.len() / 3
    }
}

fn norm(v: [f64; 3]) -> [f64; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// Sample positions along one face edge: flat interior plus `m` rim samples at
/// each end spaced by equal angle, so the rounding is smooth at any size.
fn rim_samples(h: f64, r: f64, m: usize) -> Vec<f64> {
    let mut s = Vec::new();
    // -h .. -(h - r): d = r * tan(phi), phi from 45 deg (at the edge) down to 0.
    for k in 0..=m {
        let phi = std::f64::consts::FRAC_PI_4 * (1.0 - k as f64 / m as f64);
        s.push(-(h - r) - r * phi.tan());
    }
    for k in 0..=m {
        let phi = std::f64::consts::FRAC_PI_4 * (k as f64 / m as f64);
        s.push((h - r) + r * phi.tan());
    }
    s
}

/// A box of half-extent `h` with every edge and corner rounded to radius `r`:
/// a subdivided cube whose vertices are pushed onto the offset surface of the
/// inner box `h - r`. Normals are analytic.
pub fn rounded_box(h: f64, r: f64, m: usize) -> Mesh {
    let mut mesh = Mesh::default();
    let s = rim_samples(h, r, m);
    let n = s.len();
    let inner = h - r;
    // (normal axis, u axis, v axis) with u x v = +normal for CCW front faces.
    let faces: [(usize, f64, usize, usize); 6] = [
        (0, 1.0, 1, 2),
        (0, -1.0, 2, 1),
        (1, 1.0, 2, 0),
        (1, -1.0, 0, 2),
        (2, 1.0, 0, 1),
        (2, -1.0, 1, 0),
    ];
    for (ax, sign, ua, va) in faces {
        let base = (mesh.pos.len() / 3) as u32;
        for j in 0..n {
            for i in 0..n {
                let mut p = [0.0f64; 3];
                p[ax] = sign * h;
                p[ua] = s[i];
                p[va] = s[j];
                let c = p.map(|v| v.clamp(-inner, inner));
                let d = norm([p[0] - c[0], p[1] - c[1], p[2] - c[2]]);
                let q = [c[0] + d[0] * r, c[1] + d[1] * r, c[2] + d[2] * r];
                mesh.vert(q, d);
            }
        }
        for j in 0..n - 1 {
            for i in 0..n - 1 {
                let a = base + (j * n + i) as u32;
                let b = a + 1;
                let c = a + n as u32;
                let d = c + 1;
                mesh.idx.extend_from_slice(&[a, b, d, a, d, c]);
            }
        }
    }
    mesh
}

/// Rounded-square outline, counter-clockwise seen from +z.
fn outline(half: f64, r: f64, seg: usize) -> Vec<([f64; 2], [f64; 2])> {
    let mut out = Vec::new();
    let corners = [(1.0, 1.0, 0.0), (-1.0, 1.0, 0.5), (-1.0, -1.0, 1.0), (1.0, -1.0, 1.5)];
    for (sx, sy, a0) in corners {
        let (cx, cy) = (sx * (half - r), sy * (half - r));
        for k in 0..=seg {
            let a = std::f64::consts::PI * (a0 + 0.5 * k as f64 / seg as f64);
            let (c, s) = (a.cos(), a.sin());
            out.push(([cx + c * r, cy + s * r], [c, s]));
        }
    }
    out
}

/// A sticker: a thin rounded-square slab, facing +z, sitting on z = 0, with a
/// bevelled top edge that catches the light.
pub fn sticker(half: f64, r: f64, thick: f64, bevel: f64, seg: usize) -> Mesh {
    let mut m = Mesh::default();
    let top = outline(half - bevel, r - bevel, seg);
    let rim = outline(half, r, seg);
    let k = top.len();

    // Flat top: a fan from the centre.
    let c = m.vert([0.0, 0.0, thick], [0.0, 0.0, 1.0]);
    let t0 = m.vert([top[0].0[0], top[0].0[1], thick], [0.0, 0.0, 1.0]);
    for (p, _) in top.iter().skip(1) {
        m.vert([p[0], p[1], thick], [0.0, 0.0, 1.0]);
    }
    for i in 0..k as u32 {
        m.idx.extend_from_slice(&[c, t0 + i, t0 + (i + 1) % k as u32]);
    }

    // Bevel ring.
    let b0 = (m.pos.len() / 3) as u32;
    for i in 0..k {
        let o = top[i].1;
        let n = norm([o[0], o[1], 1.0]);
        m.vert([top[i].0[0], top[i].0[1], thick], n);
        m.vert([rim[i].0[0], rim[i].0[1], thick - bevel], n);
    }
    for i in 0..k as u32 {
        let j = (i + 1) % k as u32;
        let (a, b, bn, an) = (b0 + 2 * i, b0 + 2 * i + 1, b0 + 2 * j + 1, b0 + 2 * j);
        m.idx.extend_from_slice(&[a, b, bn, a, bn, an]);
    }

    // Side wall down to the plastic.
    let s0 = (m.pos.len() / 3) as u32;
    for i in 0..k {
        let o = rim[i].1;
        let n = [o[0], o[1], 0.0];
        m.vert([rim[i].0[0], rim[i].0[1], thick - bevel], n);
        m.vert([rim[i].0[0], rim[i].0[1], 0.0], n);
    }
    for i in 0..k as u32 {
        let j = (i + 1) % k as u32;
        let (t, b, bn, tn) = (s0 + 2 * i, s0 + 2 * i + 1, s0 + 2 * j + 1, s0 + 2 * j);
        m.idx.extend_from_slice(&[b, bn, tn, b, tn, t]);
    }
    m
}

/// Both meshes, ready for QML.
pub fn meshes_json() -> Value {
    let body = rounded_box(BODY_HALF, BODY_RADIUS, 4);
    let st = sticker(STICKER_HALF, STICKER_RADIUS, STICKER_THICK, STICKER_BEVEL, 6);
    json!({
        "body": body.to_json(),
        "sticker": st.to_json(),
        "bodyHalf": BODY_HALF,
        "tris": body.tri_count() * 26 + st.tri_count() * 54,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri_normal(m: &Mesh, t: usize) -> ([f64; 3], [f64; 3]) {
        let p = |i: u32| {
            let i = i as usize * 3;
            [m.pos[i], m.pos[i + 1], m.pos[i + 2]]
        };
        let (a, b, c) = (p(m.idx[t * 3]), p(m.idx[t * 3 + 1]), p(m.idx[t * 3 + 2]));
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let i = m.idx[t * 3] as usize * 3;
        (n, [m.nrm[i], m.nrm[i + 1], m.nrm[i + 2]])
    }

    /// Every triangle must wind counter-clockwise as seen from the side its
    /// vertex normals point to, or back-face culling eats it.
    fn assert_winding(m: &Mesh) {
        for t in 0..m.tri_count() {
            let (geo, vn) = tri_normal(m, t);
            let area = (geo[0] * geo[0] + geo[1] * geo[1] + geo[2] * geo[2]).sqrt();
            if area < 1e-9 {
                continue; // degenerate sliver at a rounded corner
            }
            let d = geo[0] * vn[0] + geo[1] * vn[1] + geo[2] * vn[2];
            assert!(d > 0.0, "triangle {t} winds against its normal");
        }
    }

    #[test]
    fn body_winding_and_extent() {
        let m = rounded_box(BODY_HALF, BODY_RADIUS, 4);
        assert_winding(&m);
        let max = m.pos.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        assert!((max - BODY_HALF).abs() < 1e-9);
    }

    #[test]
    fn sticker_winding_and_size() {
        let m = sticker(STICKER_HALF, STICKER_RADIUS, STICKER_THICK, STICKER_BEVEL, 6);
        assert_winding(&m);
        let max_xy = m.pos.chunks(3).fold(0.0f64, |a, v| a.max(v[0].abs()).max(v[1].abs()));
        assert!((max_xy - STICKER_HALF).abs() < 1e-9);
        // The sticker must fit inside the flat part of the body face.
        assert!(STICKER_HALF <= BODY_HALF - BODY_RADIUS + 1e-9);
    }
}
