//! Port of the source page's `selfTest()` / `verifySequence()` blocks. They run
//! through the same `net_from()` the panel draws with, so a green run is a
//! statement about the mapping being displayed, not about a parallel
//! implementation that happens to agree with itself.

use rubik_qt::engine::cube::{
    apply_move, build_cube, inverse_of, is_solved, parse_move, Face,
};
use rubik_qt::engine::math::{apply3, cross, rot_axis, HALF_PI};
use rubik_qt::engine::net::{
    cell_grid, diagnostics, diff_of, edge_set, ids_of, net_from, net_index_of, plan_move,
    clone_cube, NET, SEAMS,
};
use rubik_qt::engine::{verify_sequence, SCRAMBLE, SOLVE};

const FLAT: [&str; 20] = {
    let mut out = ["R"; 20];
    let mut i = 0;
    while i < SCRAMBLE.len() {
        out[i] = SCRAMBLE[i];
        i += 1;
    }
    let mut j = 0;
    while j < SOLVE.len() {
        out[SCRAMBLE.len() + j] = SOLVE[j];
        j += 1;
    }
    out
};

#[test]
fn sequence_check_passes() {
    let c = verify_sequence();
    assert!(c.uniform, "every quarter turn must move exactly 9 cubies");
    assert_eq!(c.per_turn, 9);
    assert!(c.scrambled, "the scramble must actually scramble");
    assert!(c.ends_solved, "scramble + solve must be a net identity");
    assert!(c.pass);
}

#[test]
fn solved_sheet_is_fully_canonical() {
    let base = build_cube();
    let n0 = net_from(&base);
    let ids = ids_of(&n0);
    let distinct: std::collections::HashSet<i32> = ids.iter().copied().collect();
    assert_eq!(distinct.len(), 54);
    assert_eq!(n0.iter().filter(|c| c.home).count(), 54);

    let d = diagnostics(&base);
    assert_eq!(d.stickers, 54);
    assert_eq!((d.corners.ok, d.corners.n), (8, 8));
    assert_eq!((d.edges.ok, d.edges.n), (12, 12));
}

#[test]
fn net_handedness_is_outward_on_every_face() {
    for f in NET.iter() {
        let c = cross(&f.right, &f.down);
        assert_eq!(c, f.n.map(|v| -v), "{} is mirrored", f.key.key());
    }
}

#[test]
fn net_seams_fold_onto_shared_edges() {
    for (a, ea, b, eb) in SEAMS {
        assert_eq!(
            edge_set(a, ea),
            edge_set(b, eb),
            "seam {} / {}",
            NET[a].key.key(),
            NET[b].key.key()
        );
    }
}

/// Direction: a clockwise turn must drag each touched side strip onto the colour
/// the rotation matrix says it came from. Derived from the matrix alone, so it
/// still catches a sheet that is self-consistent but wound the wrong way.
#[test]
fn strip_colours_follow_the_rotation_matrix() {
    for name in ["R", "L", "U", "D", "F", "B"] {
        let mut cc = build_cube();
        apply_move(&mut cc, name);
        let nn = net_from(&cc);
        let m = parse_move(name);
        let back = rot_axis(m.axis, -m.sign as f32 * HALF_PI);
        let turned = net_index_of(Face::face_key_on_axis(m.axis, m.layer).unwrap());
        for i in 0..6 {
            if i == turned || NET[i].n[m.axis] == 0.0 {
                continue;
            }
            for t in 0..9 {
                let g = cell_grid(&NET[i], t / 3, t % 3);
                if g[m.axis] != m.layer {
                    continue;
                }
                let want = Face::of_normal(&apply3(&back, &NET[i].n)).unwrap();
                assert_eq!(
                    nn[i * 9 + t].color,
                    Some(want),
                    "{name} · {}{} want {}",
                    NET[i].key.key(),
                    t,
                    want.key()
                );
            }
        }
    }
}

#[test]
fn each_quarter_turn_permutes_twenty_stickers_bijectively() {
    let base = build_cube();
    let i0 = ids_of(&net_from(&base));
    for name in ["R", "R'", "L", "L'", "U", "U'", "D", "D'", "F", "F'", "B", "B'"] {
        let mut one = build_cube();
        apply_move(&mut one, name);
        let i1 = ids_of(&net_from(&one));
        let diff = diff_of(&i0, &i1);
        assert_eq!(diff.len(), 20, "{name}: a quarter turn moves 20 stickers");

        let distinct: std::collections::HashSet<i32> = i1.iter().copied().collect();
        assert_eq!(distinct.len(), 54, "{name}: permutation is a bijection");

        // 8, not 9: the turned face's own centre sticker rotates in place, so it
        // never changes cell. 8 + 4 strips of 3 is the 20 a quarter turn moves.
        let mut per = [0usize; 6];
        for i in &diff {
            per[i / 9] += 1;
        }
        let m = parse_move(name);
        let fidx = net_index_of(Face::face_key_on_axis(m.axis, m.layer).unwrap());
        for i in 0..6 {
            let want = if NET[i].n[m.axis] == 0.0 {
                3
            } else if i == fidx {
                8
            } else {
                0
            };
            assert_eq!(per[i], want, "{name}: per-face diff counts {per:?}");
        }

        let mut inv = one.clone();
        apply_move(&mut inv, &inverse_of(name));
        assert_eq!(ids_of(&net_from(&inv)), i0, "{name} inverse restores");

        let mut four = build_cube();
        let base_name = &name[..1];
        for _ in 0..4 {
            apply_move(&mut four, base_name);
        }
        assert_eq!(
            ids_of(&net_from(&four)),
            i0,
            "{base_name}x4 quarter turns restore"
        );
    }
}

#[test]
fn scramble_then_solve_restores_the_cube() {
    let mut cyc = build_cube();
    for name in SCRAMBLE {
        apply_move(&mut cyc, name);
    }
    let mid = diagnostics(&cyc);
    assert!(!is_solved(&cyc));
    assert!(mid.stickers < 54);
    for name in SOLVE {
        apply_move(&mut cyc, name);
    }
    let end = diagnostics(&cyc);
    assert!(is_solved(&cyc));
    assert_eq!(end.stickers, 54);
    assert_eq!((end.corners.ok, end.edges.ok), (8, 12));
}

/// A piece can stand on its home cell while twisted; the panel counts that, and
/// the gap has to be reachable or the count would be decoration.
#[test]
fn orientation_is_counted_not_position_alone() {
    let mut probe = build_cube();
    let mut gap = 0;
    for i in 0..FLAT.len() * 4 {
        apply_move(&mut probe, FLAT[i % FLAT.len()]);
        let d = diagnostics(&probe);
        gap = gap.max((d.corners.at - d.corners.ok) + (d.edges.at - d.edges.ok));
    }
    assert!(gap > 0, "no piece was ever home but misoriented");
}

#[test]
fn plan_carries_the_permutation_the_engine_will_perform() {
    for name in SCRAMBLE {
        let mut cubies = build_cube();
        let sheet = net_from(&cubies);
        let plan = plan_move(&cubies, name, &sheet);
        assert_eq!(plan.tiles.len(), 20, "{name}: 20 travelling tiles");
        for t in &plan.tiles {
            assert_ne!(t.src, t.dest);
            assert_eq!(
                plan.to[t.dest].id, sheet[t.src].id,
                "{name}: arc {}→{} must carry the sticker actually sitting in src",
                t.src, t.dest
            );
            assert_eq!(plan.to[t.dest].color, sheet[t.src].color);
        }
        // And the plan really is what the commit does.
        apply_move(&mut cubies, name);
        let after = net_from(&cubies);
        for t in &plan.tiles {
            assert_eq!(after[t.dest].id, sheet[t.src].id);
        }
    }
}

#[test]
fn cloning_a_cube_does_not_alias_the_live_state() {
    let base = build_cube();
    let mut copy = clone_cube(&base);
    apply_move(&mut copy, "R");
    assert!(is_solved(&base), "the live cube must be untouched");
    assert!(!is_solved(&copy));
}
