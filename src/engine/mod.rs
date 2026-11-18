pub mod cube;
pub mod light;
pub mod math;
pub mod net;

use cube::{apply_move, build_cube, is_solved, parse_move, Cubie};
use light::Light;
use math::{clamp01, ease_in_out_cubic, mul3, rot_axis, Mat3, TAU};

pub const SCRAMBLE: [&str; 10] = ["R", "U", "F'", "D", "L'", "B", "R'", "U'", "F", "D'"];
pub const SOLVE: [&str; 10] = ["D", "F'", "U", "R", "B'", "L", "D'", "F", "U'", "R'"];

pub const STEP: f32 = 0.5; // seconds per quarter turn
pub const HOLD: f32 = 2.0; // scrambled cube held still
pub const FREE_SPIN: f32 = 3.0; // solved free spin that closes a cycle
pub const INTRO: f32 = 2.0; // opening solved spin, start only

pub const SCR_T: f32 = (SCRAMBLE.len() as f32) * STEP;
pub const SOL_T: f32 = (SOLVE.len() as f32) * STEP;
pub const CYCLE_T: f32 = SCR_T + HOLD + SOL_T + FREE_SPIN;

/// One cycle as a flat list of quarter turns. It is a net identity, so the state
/// at ordinal n is always exactly "the first n % PER_CYCLE turns of a solved
/// cube" — repeating it cannot drift or double-apply anything.
pub fn flat() -> Vec<&'static str> {
    SCRAMBLE.into_iter().chain(SOLVE).collect()
}
pub const PER_CYCLE: usize = SCRAMBLE.len() + SOLVE.len();

pub const CAM_Z: f32 = 7.6; // camera distance; perspective = f / (CAM_Z - z)
pub const CUBIE_HALF: f32 = 0.5; // grid spacing is 1 unit
pub const BODY_HALF: f32 = 0.5; // plastic face
pub const STICKER_HALF: f32 = 0.5 * 0.82; // sticker covers 82% of the face
pub const STICKER_LIFT: f32 = 0.006; // sticker sits just proud of the plastic

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Phase {
    Intro,
    Scramble,
    Hold,
    Solve,
    Spin,
}

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Phase::Intro => "intro",
            Phase::Scramble => "scramble",
            Phase::Hold => "hold",
            Phase::Solve => "solve",
            Phase::Spin => "spin",
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Timeline {
    pub phase: Phase,
    pub n: usize,
    pub act: Option<usize>,
    pub k: f32,
    pub cycle: usize,
    pub ordinal: usize,
    pub moves_this_cycle: usize,
}

impl Timeline {
    fn new() -> Timeline {
        Timeline {
            phase: Phase::Intro,
            n: 0,
            act: None,
            k: 0.0,
            cycle: 0,
            ordinal: 0,
            moves_this_cycle: 0,
        }
    }
}

/// The slice currently turning.
#[derive(Clone, Debug)]
pub struct ActiveTurn {
    pub axis: usize,
    pub layer: i32,
    pub sign: i32,
    pub name: &'static str,
    pub k: f32,
}

/// Two clocks, deliberately:
///   `elapsed` — seconds since start, strictly increasing, owns the spin.
///   `tl`      — position inside the repeating scramble → hold → solve → free-spin
///               window, derived from `elapsed` by modulo. It holds no state of
///               its own, so wrapping it cannot snap the spin or the cube.
pub struct Engine {
    pub cubies: Vec<Cubie>,
    pub tl: Timeline,
    pub active: Option<ActiveTurn>,
    /// Monotonic quarter turns committed to the live cube.
    pub moves_committed: usize,
    pub verified_at: usize,
    pub solve_verified: bool,
    /// Cube state changed since the HUD last sampled it.
    pub state_dirty: bool,
    pub elapsed: f64,
    pub light: Light,
    pub spin: Mat3,
    pub spin_yaw: f64,
    pub spin_pitch: f64,
    pub check: SequenceCheck,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            cubies: build_cube(),
            tl: Timeline::new(),
            active: None,
            moves_committed: 0,
            verified_at: usize::MAX,
            solve_verified: false,
            state_dirty: true,
            elapsed: 0.0,
            light: Light::new(),
            spin: math::IDENT,
            spin_yaw: 0.0,
            spin_pitch: 0.0,
            check: verify_sequence(),
        }
    }

    fn timeline_at(&mut self) {
        let t = self.elapsed as f32;
        if t < INTRO {
            self.tl.phase = Phase::Intro;
            self.tl.n = 0;
            self.tl.act = None;
            self.tl.k = 0.0;
            self.tl.cycle = 0;
            self.tl.ordinal = 0;
            self.tl.moves_this_cycle = 0;
            return;
        }
        let since = t - INTRO;
        let c = (since / CYCLE_T).floor() as usize;
        let u = since - c as f32 * CYCLE_T; // cycle-local time, wraps at every boundary
        let base = c * PER_CYCLE; // ordinal of this cycle's first quarter turn
        let flat = flat();
        let _ = &flat;

        self.tl.cycle = c + 1;
        if u < SCR_T {
            let done = ((u / STEP).floor() as usize).min(SCRAMBLE.len());
            self.tl.phase = Phase::Scramble;
            self.tl.n = done;
            self.tl.ordinal = base + done;
            self.tl.act = if done < SCRAMBLE.len() { Some(done) } else { None };
            self.tl.k = ease_in_out_cubic(clamp01((u - done as f32 * STEP) / STEP));
        } else if u < SCR_T + HOLD {
            self.tl.phase = Phase::Hold;
            self.tl.n = SCRAMBLE.len();
            self.tl.ordinal = base + SCRAMBLE.len();
            self.tl.act = None;
            self.tl.k = 0.0;
        } else if u < SCR_T + HOLD + SOL_T {
            let done = (((u - SCR_T - HOLD) / STEP).floor() as usize).min(SOLVE.len());
            self.tl.phase = Phase::Solve;
            self.tl.n = done;
            self.tl.ordinal = base + SCRAMBLE.len() + done;
            self.tl.act = if done < SOLVE.len() { Some(done) } else { None };
            self.tl.k = ease_in_out_cubic(clamp01(
                (u - SCR_T - HOLD - done as f32 * STEP) / STEP,
            ));
        } else {
            // Closes the cycle at base + PER_CYCLE, which is the next cycle's
            // opening ordinal: the wrap commits nothing extra.
            self.tl.phase = Phase::Spin;
            self.tl.n = SOLVE.len();
            self.tl.ordinal = base + PER_CYCLE;
            self.tl.act = None;
            self.tl.k = 0.0;
        }
        self.tl.moves_this_cycle = self.tl.ordinal - base;
    }

    pub fn update(&mut self) {
        self.timeline_at();

        // Catch-up: a throttled tab can wake with several move boundaries already
        // behind. Each one is still committed exactly once, in order.
        while self.moves_committed < self.tl.ordinal {
            apply_move(&mut self.cubies, flat()[self.moves_committed % PER_CYCLE]);
            self.moves_committed += 1;
            self.state_dirty = true;
        }

        // The solve lands on the last turn of every cycle, so the check runs once
        // per cycle the moment that ordinal is reached.
        if self.moves_committed > 0
            && self.moves_committed % PER_CYCLE == 0
            && self.moves_committed != self.verified_at
        {
            self.verified_at = self.moves_committed;
            self.solve_verified = is_solved(&self.cubies);
            let msg = format!(
                "[cube] cycle {:03} after solve sequence: {}",
                self.tl.cycle,
                if self.solve_verified { "SOLVED" } else { "NOT SOLVED" }
            );
            if self.solve_verified {
                crate::log_line(&msg);
            } else {
                crate::log_line_err(&msg);
            }
        }

        self.active = match self.tl.act {
            None => None,
            Some(i) => {
                let name = if self.tl.phase == Phase::Scramble {
                    SCRAMBLE[i]
                } else {
                    SOLVE[i]
                };
                let m = parse_move(name);
                Some(ActiveTurn {
                    axis: m.axis,
                    layer: m.layer,
                    sign: m.sign,
                    name,
                    k: self.tl.k,
                })
            }
        };
    }

    /// Continuous yaw plus a slow, wide pitch sweep, both read straight off the
    /// monotonic `elapsed` clock. A single-axis spin cannot show all six faces: a
    /// near-vertical axis pins white and yellow to top and bottom, and a
    /// body-diagonal axis hides the three faces opposite the corner it spins on.
    /// The sweep has to reach ~60 deg or the bottom face never tilts close enough
    /// to the camera to clear the mustard threshold.
    pub fn update_spin(&mut self) {
        self.spin_yaw = self.elapsed * 0.26;
        self.spin_pitch = 1.05 * (self.elapsed * 0.085).sin();
        self.spin = mul3(
            &rot_axis(0, self.spin_pitch as f32),
            &rot_axis(1, self.spin_yaw as f32),
        );
    }

    pub fn cube_status(&self, solved: bool) -> &'static str {
        match (&self.active, self.tl.phase) {
            (Some(_), Phase::Scramble) => "SCRAMBLING",
            (Some(_), Phase::Solve) => "SOLVING",
            (Some(_), _) => "TURNING",
            (None, _) if solved => "SOLVED",
            (None, _) => "SCRAMBLED",
        }
    }
}

/// Self-check on a throwaway cube: the solve list must be the exact inverse. The
/// outcome is published so the HUD can only claim what passed.
#[derive(Copy, Clone, Debug)]
pub struct SequenceCheck {
    pub per_turn: usize,
    pub uniform: bool,
    pub scrambled: bool,
    pub ends_solved: bool,
    pub pass: bool,
}

pub fn verify_sequence() -> SequenceCheck {
    let mut test = build_cube();
    let mut ok = true;
    let mut min_turn = usize::MAX;
    for name in SCRAMBLE {
        let n = apply_move(&mut test, name);
        if n != 9 {
            ok = false;
        }
        min_turn = min_turn.min(n);
    }
    let scrambled_solved = is_solved(&test);
    for name in SOLVE {
        let n = apply_move(&mut test, name);
        if n != 9 {
            ok = false;
        }
        min_turn = min_turn.min(n);
    }
    let final_solved = is_solved(&test);
    if scrambled_solved || !final_solved {
        ok = false;
    }
    SequenceCheck {
        per_turn: if min_turn == usize::MAX { 0 } else { min_turn },
        uniform: ok,
        scrambled: !scrambled_solved,
        ends_solved: final_solved,
        pass: ok && !scrambled_solved && final_solved,
    }
}

pub fn light_phase_wrap(phase: f32) -> f32 {
    phase % TAU
}
