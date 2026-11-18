//! Everything the QML front end shows, derived from one `Engine`.
//!
//! In the softbuffer version the DNA panel owned its own transient state (the
//! in-flight plan, the commit pulse, the solved flash) and drew it. Here that
//! state machine is kept exactly as it was, but instead of drawing it publishes a
//! [`Sim::snapshot`] once per frame, and QML draws from that. The cube's pose, the
//! lighting numbers, every readout string: all of it is computed here, from the
//! same engine functions the original called, so the Qt build cannot drift from
//! the simulation any more than the original could.
//!
//! New in this version: pause, a time-scale, and a user orbit (drag the cube)
//! that composes onto the automatic spin. The orbit is folded into `eng.spin`, so
//! the lighting panel's face table keeps describing the cube you actually see.

use serde_json::{json, Value};

use crate::engine::cube::{is_solved, piece_class, Face, DIRS};
use crate::engine::light::{AMBIENT, DIFFUSE, ORBIT_RATE};
use crate::engine::math::{apply3, axis_angle, clamp01, deg, mul3, rot_axis, Mat3, HALF_PI, IDENT};
use crate::engine::net::{diagnostics, net_from, plan_move, NetCell, Plan, NET};
use crate::engine::{
    Engine, Phase, CAM_Z, CUBIE_HALF, PER_CYCLE, SCRAMBLE, SOLVE, STICKER_HALF,
};

/// Simulation step used by the headless pre-roll, as in the original `--png`.
pub const PRE_DT: f64 = 1.0 / 60.0;

pub struct Sim {
    pub eng: Engine,

    // ---- the DNA panel's transient state (ported from ui/dna.rs) ----------
    pub sheet: Vec<NetCell>,
    inflight: Option<Plan>,
    inflight_k: f32,
    inflight_ordinal: usize,
    shown_committed: usize,
    solved_shown: bool,
    pulse_cells: Vec<usize>,
    pulse_t: f32,
    solved_flash: f32,
    badge_on: bool,
    pub hover: i32,

    // ---- new controls ------------------------------------------------------
    pub paused: bool,
    pub speed: f64,
    user_yaw: f64,
    user_pitch: f64,
    vel: (f64, f64),
    dragging: bool,
    /// Bumped once per verified solve, so QML can fire a one-shot celebration
    /// by watching for a change rather than polling a flag.
    pub solves: u64,
}

impl Default for Sim {
    fn default() -> Self {
        Sim::new()
    }
}

impl Sim {
    pub fn new() -> Sim {
        let eng = Engine::new();
        let sheet = net_from(&eng.cubies);
        let solved = is_solved(&eng.cubies);
        let committed = eng.moves_committed;
        let mut s = Sim {
            eng,
            sheet,
            inflight: None,
            inflight_k: 0.0,
            inflight_ordinal: usize::MAX,
            shown_committed: committed,
            solved_shown: solved,
            pulse_cells: Vec::new(),
            pulse_t: 1.0,
            solved_flash: 9.0,
            badge_on: false,
            hover: -1,
            paused: false,
            speed: 1.0,
            user_yaw: 0.0,
            user_pitch: 0.0,
            vel: (0.0, 0.0),
            dragging: false,
            solves: 0,
        };
        s.eng.update();
        s.compose_spin();
        s
    }

    /// One frame of wall-clock time. Mirrors the original window loop: advance
    /// the clock, animate the panel's effects, then update the engine, sync the
    /// panel, move the light and recompute the spin.
    pub fn step(&mut self, dt_wall: f64) {
        let dt = dt_wall.clamp(0.0, 0.1); // clamp after a stall, as the page does
        let h = if self.paused { 0.0 } else { dt * self.speed };
        self.advance(h);
        self.coast(dt);
        self.compose_spin();
    }

    /// Steps the simulation at 60 Hz up to `at` seconds, without drawing — the
    /// same pre-roll the original `--png` used, so transients that only exist
    /// because the clock passed through them (a commit pulse, an in-flight plan)
    /// are present in the captured frame.
    pub fn prestep(&mut self, at: f64) {
        let mut t = self.eng.elapsed;
        while t < at - 1e-9 {
            // The last step is short: overshooting would commit a quarter turn
            // that has not happened yet at `at`, and commits are monotonic.
            let h = PRE_DT.min(at - t);
            t += h;
            self.advance_to(t, h);
        }
        self.compose_spin();
    }

    fn advance(&mut self, h: f64) {
        let t = self.eng.elapsed + h;
        self.advance_to(t, h);
    }

    fn advance_to(&mut self, t: f64, h: f64) {
        let before = self.eng.verified_at;
        self.eng.elapsed = t;
        self.animate(h as f32);
        self.eng.update();
        self.tick_panel();
        self.eng.light.update(h as f32);
        self.eng.update_spin();
        if self.eng.verified_at != before && self.eng.solve_verified {
            self.solves += 1;
        }
    }

    /// Inertia for the user orbit: after a fling the cube coasts and settles.
    fn coast(&mut self, dt: f64) {
        if self.dragging {
            return;
        }
        let k = (-3.2 * dt).exp();
        self.vel = (self.vel.0 * k, self.vel.1 * k);
        self.user_yaw += self.vel.0 * dt;
        self.user_pitch = (self.user_pitch + self.vel.1 * dt).clamp(-1.35, 1.35);
    }

    /// Automatic spin (yaw + pitch sweep, from the engine) with the user's own
    /// orbit composed in front of it, in view space.
    fn compose_spin(&mut self) {
        self.eng.update_spin();
        let user = mul3(
            &rot_axis(0, self.user_pitch as f32),
            &rot_axis(1, self.user_yaw as f32),
        );
        self.eng.spin = mul3(&user, &self.eng.spin);
    }

    // ---- user controls -------------------------------------------------------
    pub fn orbit_by(&mut self, dx: f64, dy: f64) {
        self.dragging = true;
        self.user_yaw += dx * 0.0085;
        self.user_pitch = (self.user_pitch + dy * 0.0085).clamp(-1.35, 1.35);
        self.compose_spin();
    }

    /// End of a drag; `vx`, `vy` are the release velocity in px/s.
    pub fn release(&mut self, vx: f64, vy: f64) {
        self.dragging = false;
        self.vel = ((vx * 0.0085).clamp(-6.0, 6.0), (vy * 0.0085).clamp(-6.0, 6.0));
    }

    pub fn reset_view(&mut self) {
        self.user_yaw = 0.0;
        self.user_pitch = 0.0;
        self.vel = (0.0, 0.0);
        self.compose_spin();
    }

    // ---- the DNA panel's sync, verbatim from DnaPanel::tick / animate ----------
    fn tick_panel(&mut self) {
        let eng = &self.eng;
        if eng.moves_committed != self.shown_committed {
            let next = net_from(&eng.cubies);
            let changed: Vec<usize> = (0..54).filter(|i| next[*i].id != self.sheet[*i].id).collect();
            self.sheet = next;
            if !changed.is_empty() {
                self.pulse_cells = changed;
                self.pulse_t = 0.0;
            }
            self.inflight = None;
            self.shown_committed = eng.moves_committed;
            let now_solved = is_solved(&eng.cubies);
            if now_solved && !self.solved_shown {
                self.solved_flash = 0.0;
                self.badge_on = true;
            } else if !now_solved {
                self.badge_on = false;
            }
            self.solved_shown = now_solved;
        }

        if let Some(a) = eng.active.clone() {
            let stale = match &self.inflight {
                None => true,
                Some(p) => p.name != a.name || self.inflight_ordinal != eng.moves_committed,
            };
            if stale {
                self.inflight = Some(plan_move(&eng.cubies, a.name, &self.sheet));
                self.inflight_ordinal = eng.moves_committed;
            }
            self.inflight_k = a.k;
        } else if self.inflight.is_some() {
            self.inflight = None;
        }
    }

    fn animate(&mut self, dt: f32) {
        self.pulse_t = (self.pulse_t + dt / 0.62).min(1.0);
        self.solved_flash += dt;
        if self.badge_on && (self.solved_flash > 2.4 || !self.solved_shown) {
            self.badge_on = false;
        }
    }

    /// Hover from the 3D view: a picked sticker (`slot` >= 0) selects exactly its
    /// net cell; a hit on bare plastic (`slot` < 0) selects the cubie's first
    /// sticker in the net. Returns the net cell, or -1.
    pub fn hover_sticker(&mut self, cubie: i32, slot: i32) -> i32 {
        let cub = cubie as usize;
        let cell = self
            .sheet
            .iter()
            .position(|c| c.cub == cub && c.slot == slot)
            .or_else(|| self.sheet.iter().position(|c| c.cub == cub));
        self.hover = cell.map(|c| c as i32).unwrap_or(-1);
        self.hover
    }

    /// The hovered net cell's cubie, which the 3D view outlines.
    pub fn hovered_cubie(&self) -> Option<usize> {
        if self.hover < 0 {
            return None;
        }
        self.sheet.get(self.hover as usize).map(|c| c.cub)
    }

    // ---- snapshot ------------------------------------------------------------

    /// Everything QML needs for one frame, as a JSON value (QtBridge hands it to
    /// QML as a plain JS object).
    pub fn snapshot(&self) -> Value {
        let eng = &self.eng;
        json!({
            "t": eng.elapsed,
            "paused": self.paused,
            "speed": self.speed,
            "solves": self.solves,
            "spin": quat(&eng.spin),
            "spinM": eng.spin.to_vec(),
            "cubies": self.cubie_xforms(),
            "hoverCubie": self.hovered_cubie().map(|c| c as i64).unwrap_or(-1),
            "light": self.light_json(),
            "tele": self.tele_json(),
            "timeline": self.timeline_json(),
            "status": self.status_json(),
            "dna": self.dna_json(),
        })
    }

    /// 26 x [px, py, pz, qw, qx, qy, qz]: each cubie's position and orientation in
    /// cube space, with the partial slice turn already applied. The spin goes on
    /// the parent node, so world = spin * (extra * rot), exactly the original's
    /// `model = spin * extra; rot_w = model * rot`.
    fn cubie_xforms(&self) -> Vec<f64> {
        let eng = &self.eng;
        let mut out = Vec::with_capacity(26 * 7);
        for c in eng.cubies.iter() {
            let extra = slice_extra(eng, c.p);
            let pos = apply3(&extra, &c.p.map(|v| v as f32));
            let q = quat(&mul3(&extra, &c.rot));
            out.extend(pos.iter().map(|v| *v as f64));
            out.extend(q.iter());
        }
        out
    }

    fn light_json(&self) -> Value {
        let l = &self.eng.light;
        let faces: Vec<Value> = DIRS
            .iter()
            .map(|d| {
                let n = apply3(&self.eng.spin, &d.n());
                let raw = n[0] * l.dir[0] + n[1] * l.dir[1] + n[2] * l.dir[2];
                let (axis, at) = d.axis_at();
                json!({
                    "key": d.key(),
                    "label": format!("{}{}", if at > 0 { "+" } else { "\u{2212}" }, ["X", "Y", "Z"][axis]),
                    "color": d.color().css(),
                    "raw": raw,
                    "lit": l.lambert(&n),
                    "b": l.brightness(&n),
                    "text": format!("{} · {:.2}", signed3(raw), l.brightness(&n)),
                })
            })
            .collect();
        json!({
            "dir": l.dir.to_vec(),
            "dirText": l.dir.iter().map(|v| signed3(*v)).collect::<Vec<_>>(),
            "q": light_quat(&l.dir),
            "orbit": l.orbit,
            "phase": l.phase,
            "az0": l.az0,
            "elev": l.elev,
            "elevCos": l.elev_cos,
            "faces": faces,
            "meta": [
                ["MODEL", "lambertian diffuse", false],
                ["AMB / DIF", format!("{AMBIENT:.2} / {DIFFUSE:.2}"), false],
                ["LIGHT", "directional · dir × 2.30", false],
                ["PHASE", if l.orbit {
                    format!("{:.1}° · orbit {:.0}°/s", deg(l.phase), deg(ORBIT_RATE))
                } else {
                    "— · static".to_string()
                }, true],
                ["CUBE", format!(
                    "yaw {:.1}° · pitch {:.1}°",
                    deg(self.eng.spin_yaw as f32) % 360.0,
                    deg(self.eng.spin_pitch as f32)
                ), false],
                ["CAMERA", format!("fixed · z {CAM_Z:.1} · identity"), false],
            ],
        })
    }

    fn tele_json(&self) -> Value {
        let eng = &self.eng;
        let total_moves = SCRAMBLE.len() + SOLVE.len();
        let full = eng.tl.moves_this_cycle >= total_moves;
        json!([
            ["LIGHT DIR", format!("({:.3}, {:.3}, {:.3})", eng.light.dir[0], eng.light.dir[1], eng.light.dir[2]), false],
            ["SHADE", format!("{AMBIENT:.2} + {DIFFUSE:.2} × max(0, n·l)"), false],
            ["BODY", "rgb(26, 26, 34)", false],
            ["STICKER", format!("{}% of face", (STICKER_HALF / CUBIE_HALF * 100.0).round() as i32), false],
            ["CUBIES", eng.cubies.len().to_string(), false],
            ["STICKERS", count_stickers(eng).to_string(), false],
            ["QUADS / FRAME", visible_quads(eng).to_string(), false],
            ["MOVES", format!("{:02} / {}", eng.tl.moves_this_cycle, total_moves), full],
            ["TIME SCALE", if self.paused { "paused".to_string() } else { format!("{:.2}×", self.speed) }, self.paused],
        ])
    }

    fn timeline_json(&self) -> Value {
        let tl = &self.eng.tl;
        let lane = |li: usize, len: usize| -> (usize, i64) {
            let (done, act) = match (li, tl.phase) {
                (0, Phase::Scramble) => (tl.n, tl.act),
                (0, Phase::Intro) => (0, None),
                (0, _) => (len, None),
                (1, Phase::Solve) => (tl.n, tl.act),
                (1, Phase::Spin) => (len, None),
                _ => (0, None),
            };
            (done, act.map(|a| a as i64).unwrap_or(-1))
        };
        let (sd, sa) = lane(0, SCRAMBLE.len());
        let (vd, va) = lane(1, SOLVE.len());
        json!({
            "cycle": tl.cycle.max(1),
            "phase": tl.phase.name(),
            "scramble": SCRAMBLE.to_vec(),
            "solve": SOLVE.to_vec(),
            "scrDone": sd, "scrAct": sa,
            "solDone": vd, "solAct": va,
            "k": tl.k,
        })
    }

    fn status_json(&self) -> Value {
        let eng = &self.eng;
        let turning = matches!(eng.tl.phase, Phase::Scramble | Phase::Solve);
        let solved = is_solved(&eng.cubies);
        let off = off_stickers(eng);
        let total = count_stickers(eng);
        let (tone, text) = if turning {
            let (list, verb) = if eng.tl.phase == Phase::Scramble {
                (&SCRAMBLE[..], "SCRAMBLING")
            } else {
                (&SOLVE[..], "SOLVING")
            };
            ("busy", format!("{verb} · MOVE {:02}/{:02}", (eng.tl.n + 1).min(list.len()), list.len()))
        } else if solved {
            ("ok", "SOLVED · FREE SPIN".to_string())
        } else {
            ("idle", "SCRAMBLED · HOLD".to_string())
        };
        json!({
            "tone": tone,
            "text": text,
            "state": format!("STATE · {} ({}/{} OFF)", if solved { "SOLVED" } else { "UNSOLVED" }, off, total),
            "check": if eng.check.pass {
                format!("✓ self-check · {} moves → solved · {} cubies/turn", PER_CYCLE, eng.check.per_turn)
            } else {
                "✗ self-check failed".to_string()
            },
            "checkPass": eng.check.pass,
            "cube": eng.cube_status(self.solved_shown),
        })
    }

    fn dna_json(&self) -> Value {
        let eng = &self.eng;
        let sheet: Vec<&str> = self.sheet.iter().map(|c| c.color.map(|f| f.key()).unwrap_or("")).collect();

        let inflight = self.inflight.as_ref().map(|p| {
            let tiles: Vec<Value> = p
                .tiles
                .iter()
                .map(|t| json!([t.src, t.dest, t.color.map(|f| f.key()).unwrap_or(""), t.same_face]))
                .collect();
            let moved: Vec<usize> = (0..54).filter(|i| p.moved[*i]).collect();
            json!({ "face": p.face_idx, "cw": p.cw, "tiles": tiles, "moved": moved })
        });

        // ---- readout: ACTIVE TRANSFORMATION ----
        let note = if eng.active.is_some() {
            format!(
                "{} · {}/{}",
                eng.tl.phase.name(),
                eng.tl.act.map(|a| a + 1).unwrap_or(0),
                if eng.tl.phase == Phase::Scramble { SCRAMBLE.len() } else { SOLVE.len() }
            )
        } else if self.solved_shown {
            "idle · state committed".to_string()
        } else {
            "idle · holding".to_string()
        };
        let (name, axis, dir, angle) = match &self.inflight {
            Some(pl) => {
                let ang = pl.sign as f32 * 90.0 * clamp01(self.inflight_k);
                (
                    pl.name.to_string(),
                    format!("{} · layer {}", ["X", "Y", "Z"][pl.axis], if pl.layer > 0 { "+1" } else { "\u{2212}1" }),
                    if pl.cw { "clockwise" } else { "counterclockwise" }.to_string(),
                    format!(
                        "{}{:.1}° / {}90°",
                        if ang < 0.0 { "\u{2212}" } else { "+" },
                        ang.abs(),
                        if pl.sign < 0 { "\u{2212}" } else { "+" }
                    ),
                )
            }
            None => ("—".into(), "—".into(), "—".into(), "0.0°".into()),
        };

        // ---- readout: CUBE INTEGRITY ----
        let d = diagnostics(&eng.cubies);

        json!({
            "sheet": sheet,
            "inflight": inflight,
            "k": self.inflight_k,
            "pulse": { "cells": self.pulse_cells, "t": self.pulse_t },
            "flash": self.solved_flash,
            "badge": self.badge_on,
            "solvedShown": self.solved_shown,
            "inspect": self.inspect_json(),
            "readout": {
                "note": note, "name": name, "axis": axis, "dir": dir, "angle": angle,
                "live": self.inflight.is_some(),
            },
            "integrity": [
                ["STICKERS", format!("{:02} / 54", d.stickers), d.stickers as f64 / 54.0],
                ["CORNERS", format!("{} / {}", d.corners.ok, d.corners.n), d.corners.ok as f64 / d.corners.n as f64],
                ["EDGES", format!("{} / {}", d.edges.ok, d.edges.n), d.edges.ok as f64 / d.edges.n as f64],
            ],
        })
    }

    /// The hover inspector's three lines, as DnaPanel::draw_inspect wrote them.
    fn inspect_json(&self) -> Value {
        if self.hover < 0 {
            return Value::Null;
        }
        let idx = self.hover as usize;
        let Some(cell) = self.sheet.get(idx) else { return Value::Null };
        let (fi, row, col) = (idx / 9, (idx % 9) / 3, idx % 3);
        let cub = &self.eng.cubies[cell.cub];
        let (deg, ax) = axis_angle(&cub.rot);
        json!({
            "idx": idx,
            "lines": [
                format!("{} · r{} c{}", NET[fi].key.key(), row + 1, col + 1),
                format!("cubie ({}) · {}", fmt3(&cub.home), piece_class(&cub.home)),
                match ax {
                    Some(a) => format!("rot {:.0}° about ({:.2},{:.2},{:.2})", deg, a[0], a[1], a[2]),
                    None => "rot identity".to_string(),
                },
            ],
        })
    }
}

/// The static description of the net and palette, sent to QML once.
pub fn static_info() -> Value {
    let faces: Vec<Value> = NET
        .iter()
        .map(|f| json!({ "key": f.key.key(), "fc": f.fc, "fr": f.fr, "tag": f.tag }))
        .collect();
    let mut palette = serde_json::Map::new();
    for d in DIRS {
        palette.insert(d.key().to_string(), json!(d.color().css()));
    }
    // Stickers each cubie carries in its own frame:
    // [cubie, face key, nx, ny, nz, slot] — `slot` indexes the cubie's face table,
    // the same handle the net uses, so a picked 3D sticker maps to its net cell.
    let mut stickers = Vec::new();
    let eng = Engine::new();
    for (i, c) in eng.cubies.iter().enumerate() {
        for (slot, s) in c.faces.iter().enumerate() {
            if let Some(col) = s.color {
                stickers.push(json!([i, col.key(), s.n[0], s.n[1], s.n[2], slot]));
            }
        }
    }
    json!({
        "net": faces,
        "palette": palette,
        "faceOrder": DIRS.iter().map(|d| d.key()).collect::<Vec<_>>(),
        "swatches": ["U", "F", "R", "B", "L", "D"],
        "stickers": stickers,
        "body": crate::engine::cube::BODY_COLOR.css(),
        "camZ": CAM_Z,
        "perCycle": PER_CYCLE,
    })
}

// ---- helpers ------------------------------------------------------------------

fn slice_extra(eng: &Engine, p: [i32; 3]) -> Mat3 {
    match &eng.active {
        Some(a) if p[a.axis] == a.layer => rot_axis(a.axis, a.sign as f32 * HALF_PI * a.k),
        _ => IDENT,
    }
}

/// Row-major rotation matrix to a unit quaternion, `[w, x, y, z]` — the order
/// `Qt.quaternion(w, x, y, z)` takes.
pub fn quat(m: &Mat3) -> [f64; 4] {
    let m: Vec<f64> = m.iter().map(|v| *v as f64).collect();
    let tr = m[0] + m[4] + m[8];
    let q = if tr > 0.0 {
        let s = (tr + 1.0).sqrt() * 2.0;
        [0.25 * s, (m[7] - m[5]) / s, (m[2] - m[6]) / s, (m[3] - m[1]) / s]
    } else if m[0] > m[4] && m[0] > m[8] {
        let s = (1.0 + m[0] - m[4] - m[8]).sqrt() * 2.0;
        [(m[7] - m[5]) / s, 0.25 * s, (m[1] + m[3]) / s, (m[2] + m[6]) / s]
    } else if m[4] > m[8] {
        let s = (1.0 + m[4] - m[0] - m[8]).sqrt() * 2.0;
        [(m[2] - m[6]) / s, (m[1] + m[3]) / s, 0.25 * s, (m[5] + m[7]) / s]
    } else {
        let s = (1.0 + m[8] - m[0] - m[4]).sqrt() * 2.0;
        [(m[3] - m[1]) / s, (m[2] + m[6]) / s, (m[5] + m[7]) / s, 0.25 * s]
    };
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    [q[0] / l, q[1] / l, q[2] / l, q[3] / l]
}

/// Orientation for a Qt Quick 3D `DirectionalLight`, which shines along its
/// local -z. The engine's `dir` points FROM the surface TOWARD the source, so
/// the node's +z is turned onto `dir`.
pub fn light_quat(d: &[f32; 3]) -> [f64; 4] {
    let (x, y, z) = (d[0] as f64, d[1] as f64, d[2] as f64);
    let w = 1.0 + z;
    if w < 1e-6 {
        return [0.0, 0.0, 1.0, 0.0];
    }
    // axis = (0,0,1) x d
    let q = [w, -y, x, 0.0];
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
    [q[0] / l, q[1] / l, q[2] / l, 0.0]
}

/// Faces that pass the original renderer's backface test this frame — the
/// "quads / frame" the painter's algorithm would have sorted and drawn.
pub fn visible_quads(eng: &Engine) -> usize {
    let mut n = 0;
    for c in eng.cubies.iter() {
        let model = mul3(&eng.spin, &slice_extra(eng, c.p));
        let rot_w = mul3(&model, &c.rot);
        let pos = apply3(&model, &c.p.map(|v| v as f32));
        for fc in c.faces.iter() {
            let nn = apply3(&rot_w, &fc.n);
            let ctr = [pos[0] + CUBIE_HALF * nn[0], pos[1] + CUBIE_HALF * nn[1], pos[2] + CUBIE_HALF * nn[2]];
            if nn[0] * -ctr[0] + nn[1] * -ctr[1] + nn[2] * (CAM_Z - ctr[2]) > 0.0 {
                n += 1;
            }
        }
    }
    n
}

/// Stickers whose current face is not the colour they carry (`c.rot` only —
/// the free spin is irrelevant).
pub fn off_stickers(eng: &Engine) -> usize {
    let mut off = 0;
    for c in eng.cubies.iter() {
        for f in c.faces.iter() {
            let Some(col) = f.color else { continue };
            let n = apply3(&c.rot, &f.n);
            if Face::of_normal(&n) != Some(col) {
                off += 1;
            }
        }
    }
    off
}

pub fn count_stickers(eng: &Engine) -> usize {
    eng.cubies.iter().flat_map(|c| c.faces.iter()).filter(|f| f.color.is_some()).count()
}

fn signed3(v: f32) -> String {
    format!("{}{:.3}", if v < 0.0 { "\u{2212}" } else { "+" }, v.abs())
}

fn fmt3(v: &[i32; 3]) -> String {
    let s = |x: i32| format!("{}{}", if x < 0 { "\u{2212}" } else if x > 0 { "+" } else { " " }, x.abs());
    format!("{}, {}, {}", s(v[0]), s(v[1]), s(v[2]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{CYCLE_T, INTRO};

    fn rotate_by_quat(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
        let (w, x, y, z) = (q[0], q[1], q[2], q[3]);
        // v' = v + 2w(u x v) + 2u x (u x v), u = (x, y, z)
        let c1 = [y * v[2] - z * v[1], z * v[0] - x * v[2], x * v[1] - y * v[0]];
        let c2 = [y * c1[2] - z * c1[1], z * c1[0] - x * c1[2], x * c1[1] - y * c1[0]];
        [v[0] + 2.0 * (w * c1[0] + c2[0]), v[1] + 2.0 * (w * c1[1] + c2[1]), v[2] + 2.0 * (w * c1[2] + c2[2])]
    }

    #[test]
    fn quat_matches_matrix() {
        let m = mul3(&rot_axis(0, 0.7), &mul3(&rot_axis(1, -2.1), &rot_axis(2, 2.9)));
        let q = quat(&m);
        for v in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.3, -0.4, 0.8]] {
            let a = apply3(&m, &[v[0] as f32, v[1] as f32, v[2] as f32]);
            let b = rotate_by_quat(q, v);
            for k in 0..3 {
                assert!((a[k] as f64 - b[k]).abs() < 1e-5, "{a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn light_quat_turns_plus_z_onto_dir() {
        for d in [[0.26f32, 0.14, 0.955], [-0.6, 0.2, -0.77], [0.0, 1.0, 0.0]] {
            let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            let d = [d[0] / l, d[1] / l, d[2] / l];
            let r = rotate_by_quat(light_quat(&d), [0.0, 0.0, 1.0]);
            for k in 0..3 {
                assert!((r[k] - d[k] as f64).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn prestep_through_a_cycle_counts_one_verified_solve() {
        let mut s = Sim::new();
        s.prestep((INTRO + CYCLE_T) as f64 - 0.5); // inside the closing free spin
        assert_eq!(s.solves, 1);
        assert!(is_solved(&s.eng.cubies));
        let snap = s.snapshot();
        assert_eq!(snap["status"]["tone"], "ok");
        assert_eq!(snap["cubies"].as_array().unwrap().len(), 26 * 7);
        assert_eq!(snap["dna"]["sheet"].as_array().unwrap().len(), 54);
    }

    #[test]
    fn hovering_a_3d_sticker_selects_its_net_cell() {
        let mut s = Sim::new();
        s.prestep(5.3); // scrambled a little, so cells and cubies are shuffled
        for (idx, cell) in s.sheet.clone().iter().enumerate() {
            assert_eq!(s.hover_sticker(cell.cub as i32, cell.slot), idx as i32);
        }
        // bare plastic still lands on one of that cubie's cells
        let cell = s.hover_sticker(0, -1);
        assert_eq!(s.sheet[cell as usize].cub, 0);
    }

    #[test]
    fn pause_freezes_the_clock_and_speed_scales_it() {
        let mut s = Sim::new();
        s.paused = true;
        s.step(0.05);
        assert_eq!(s.eng.elapsed, 0.0);
        s.paused = false;
        s.speed = 2.0;
        s.step(0.05);
        assert!((s.eng.elapsed - 0.1).abs() < 1e-12);
    }

    #[test]
    fn user_orbit_reaches_the_lighting_panel() {
        let mut s = Sim::new();
        let before = s.snapshot()["light"]["faces"][4]["raw"].as_f64().unwrap();
        s.orbit_by(180.0, 0.0); // ~1.5 rad of yaw
        let after = s.snapshot()["light"]["faces"][4]["raw"].as_f64().unwrap();
        assert!((before - after).abs() > 0.3, "face table ignored the user orbit");
        s.reset_view();
        let back = s.snapshot()["light"]["faces"][4]["raw"].as_f64().unwrap();
        assert!((before - back).abs() < 1e-6);
    }
}
