//! The unfolded net, read straight out of the cube's own state.
//!
//! The engine stores stickers in cubie-local space — `faces[].n` / `faces[].c`
//! ride on the piece and `c.rot` maps them into cube space — so the net is a
//! read of that state rather than a second copy of it: for each of the 54 net
//! cells, take the cubie holding that grid position and pick the one sticker
//! whose rotated normal equals the face's outward normal. That is the canonical
//! puzzle layout, independent of the camera and of the free spin.

use std::collections::HashMap;

use super::cube::{apply_move, oriented, parse_move, piece_class, Cubie, Face};
use super::math::{apply3, Vec3};

/// The unfolded sheet. `right`/`down` give each face its conventional reading: U
/// above F with its back row toward B, the L F R B band left to right, D below F
/// with its far row toward B. `right × down = −n` for every entry, so each face
/// is drawn as seen from outside the cube and none of them is mirrored.
#[derive(Copy, Clone)]
pub struct NetFace {
    pub key: Face,
    pub fc: usize,
    pub fr: usize,
    pub n: Vec3,
    pub right: Vec3,
    pub down: Vec3,
    pub tag: &'static str,
}

pub const NET: [NetFace; 6] = [
    NetFace { key: Face::U, fc: 1, fr: 0, n: [0.0, 1.0, 0.0], right: [1.0, 0.0, 0.0], down: [0.0, 0.0, 1.0], tag: "+Y" },
    NetFace { key: Face::L, fc: 0, fr: 1, n: [-1.0, 0.0, 0.0], right: [0.0, 0.0, 1.0], down: [0.0, -1.0, 0.0], tag: "\u{2212}X" },
    NetFace { key: Face::F, fc: 1, fr: 1, n: [0.0, 0.0, 1.0], right: [1.0, 0.0, 0.0], down: [0.0, -1.0, 0.0], tag: "+Z" },
    NetFace { key: Face::R, fc: 2, fr: 1, n: [1.0, 0.0, 0.0], right: [0.0, 0.0, -1.0], down: [0.0, -1.0, 0.0], tag: "+X" },
    NetFace { key: Face::B, fc: 3, fr: 1, n: [0.0, 0.0, -1.0], right: [-1.0, 0.0, 0.0], down: [0.0, -1.0, 0.0], tag: "\u{2212}Z" },
    NetFace { key: Face::D, fc: 1, fr: 2, n: [0.0, -1.0, 0.0], right: [1.0, 0.0, 0.0], down: [0.0, 0.0, -1.0], tag: "\u{2212}Y" },
];

pub fn net_index_of(key: Face) -> usize {
    NET.iter().position(|f| f.key == key).unwrap()
}

#[derive(Copy, Clone, Debug)]
pub struct NetCell {
    /// Stable sticker handle: cubie index * 6 + slot in that cubie's fixed face
    /// table. Differencing two nets recovers the permutation a turn performs.
    pub id: i32,
    pub cub: usize,
    pub slot: i32,
    pub color: Option<Face>,
    pub home: bool,
}

/// Cell `(r, c)` of one face, in grid coordinates. `n`, `right` and `down` are
/// mutually orthogonal axis vectors, so this lands exactly on a cubie position.
pub fn cell_grid(f: &NetFace, r: usize, c: usize) -> [i32; 3] {
    let mut g = [0i32; 3];
    for k in 0..3 {
        g[k] = f.n[k] as i32
            + f.right[k] as i32 * (c as i32 - 1)
            + f.down[k] as i32 * (r as i32 - 1);
    }
    g
}

fn rotated_normal_key(cub: &Cubie, slot: usize) -> [i32; 3] {
    let nn = apply3(&cub.rot, &cub.faces[slot].n);
    [nn[0].round() as i32, nn[1].round() as i32, nn[2].round() as i32]
}

/// The 54 net cells read straight out of a cubie list.
pub fn net_from(list: &[Cubie]) -> Vec<NetCell> {
    let mut cells = Vec::with_capacity(54);
    for f in NET.iter() {
        let want = f.n.map(|v| v as i32);
        for r in 0..3 {
            for c in 0..3 {
                let g = cell_grid(f, r, c);
                let idx = list
                    .iter()
                    .position(|x| x.p == g)
                    .expect("net cell has a cubie under it");
                let cub = &list[idx];
                let mut slot = -1i32;
                for s in 0..cub.faces.len() {
                    if rotated_normal_key(cub, s) == want {
                        slot = s as i32;
                        break;
                    }
                }
                let color = if slot < 0 {
                    None
                } else {
                    cub.faces[slot as usize].color
                };
                cells.push(NetCell {
                    id: (idx as i32) * 6 + slot,
                    cub: idx,
                    slot,
                    color,
                    home: color == Some(f.key),
                });
            }
        }
    }
    cells
}

/// Shallow enough to be cheap: `faces` and `home` are immutable per cubie, so
/// only the two fields a turn rewrites need their own copies.
pub fn clone_cube(list: &[Cubie]) -> Vec<Cubie> {
    list.to_vec()
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Bucket {
    pub ok: usize,
    pub at: usize,
    pub n: usize,
}

#[derive(Clone, Debug)]
pub struct Diagnostics {
    pub cells: Vec<NetCell>,
    pub stickers: usize,
    pub corners: Bucket,
    pub edges: Bucket,
}

/// `at` counts pieces standing on their home position, `ok` those that are both
/// there and untwisted — the gap between the two is exactly what a colour-only
/// check would miss.
pub fn diagnostics(list: &[Cubie]) -> Diagnostics {
    let cells = net_from(list);
    let stickers = cells.iter().filter(|c| c.home).count();
    let mut st = Diagnostics {
        cells,
        stickers,
        corners: Bucket::default(),
        edges: Bucket::default(),
    };
    for c in list {
        let cls = piece_class(&c.home);
        let bucket = match cls {
            "corner" => &mut st.corners,
            "edge" => &mut st.edges,
            _ => continue,
        };
        bucket.n += 1;
        if c.p == c.home {
            bucket.at += 1;
            if oriented(&c.rot) {
                bucket.ok += 1;
            }
        }
    }
    st
}

#[derive(Copy, Clone, Debug)]
pub struct Tile {
    pub dest: usize,
    pub src: usize,
    pub color: Option<Face>,
    pub same_face: bool,
}

/// The turn about to happen, as a sticker permutation.
#[derive(Clone, Debug)]
pub struct Plan {
    pub name: &'static str,
    pub axis: usize,
    pub layer: i32,
    pub sign: i32,
    /// Clockwise as seen from outside the turned face: a positive rotation about
    /// +axis is counterclockwise from that side only when the layer is on +axis.
    pub cw: bool,
    pub face_idx: usize,
    pub tiles: Vec<Tile>,
    pub moved: [bool; 54],
    pub to: Vec<NetCell>,
}

pub fn plan_move(cubies: &[Cubie], name: &'static str, sheet: &[NetCell]) -> Plan {
    let mut scratch = clone_cube(cubies);
    apply_move(&mut scratch, name);
    let to = net_from(&scratch);
    let src_of: HashMap<i32, usize> = sheet.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

    let mut tiles = Vec::new();
    let mut moved = [false; 54];
    for i in 0..54 {
        if to[i].id == sheet[i].id {
            continue;
        }
        let from = src_of[&to[i].id];
        tiles.push(Tile {
            dest: i,
            src: from,
            color: to[i].color,
            same_face: (i / 9) == (from / 9),
        });
        moved[i] = true;
        moved[from] = true;
    }

    let m = parse_move(name);
    Plan {
        name,
        axis: m.axis,
        layer: m.layer,
        sign: m.sign,
        cw: m.sign * m.layer < 0,
        face_idx: net_index_of(Face::face_key_on_axis(m.axis, m.layer).unwrap()),
        tiles,
        moved,
        to,
    }
}

/// The three grid positions along one edge of a face as the sheet draws it.
pub fn edge_set(fi: usize, edge: &str) -> String {
    let mut out = Vec::new();
    for t in 0..3 {
        let r = if edge == "top" { 0 } else if edge == "bottom" { 2 } else { t };
        let c = if edge == "left" { 0 } else if edge == "right" { 2 } else { t };
        out.push(
            cell_grid(&NET[fi], r, c)
                .map(|v| v.to_string())
                .join(","),
        );
    }
    out.sort();
    out.join(" ")
}

/// The seams of the sheet: which drawn edge of which face meets which.
pub const SEAMS: [(usize, &str, usize, &str); 5] = [
    (0, "bottom", 2, "top"),
    (1, "right", 2, "left"),
    (2, "right", 3, "left"),
    (3, "right", 4, "left"),
    (2, "bottom", 5, "top"),
];

pub fn ids_of(cells: &[NetCell]) -> Vec<i32> {
    cells.iter().map(|c| c.id).collect()
}

pub fn diff_of(a: &[i32], b: &[i32]) -> Vec<usize> {
    (0..a.len()).filter(|i| a[*i] != b[*i]).collect()
}
