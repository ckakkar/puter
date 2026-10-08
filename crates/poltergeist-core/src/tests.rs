use super::*;
use crate::collision::{box_feature, polygon_vertex};
use crate::history::History;

const DT: f32 = 1.0 / 60.0;

fn run(w: &mut World, seconds: f32) {
    for _ in 0..(seconds / DT).round() as usize {
        w.step(DT);
    }
}
fn floor(w: &mut World) -> u32 {
    w.add(
        Shape::Box {
            half: Vec2::new(20.0, 0.5),
        },
        Vec2::new(0.0, -0.5),
        0.0,
    )
}

#[test]
fn free_fall_matches_semi_implicit_integration() {
    let mut w = World::new();
    w.add(Shape::Circle { radius: 0.5 }, Vec2::new(0.0, 20.0), 1.0);
    for _ in 0..60 {
        w.step(1.0 / 60.0);
    }
    let b = &w.bodies[0];
    assert!((b.velocity.y + 9.81).abs() < 0.001);
    assert!((b.position.y - 15.054).abs() < 0.003);
}
#[test]
fn circle_collision_conserves_momentum() {
    let mut w = World::new();
    w.gravity = Vec2::ZERO;
    w.add(Shape::Circle { radius: 0.5 }, Vec2::new(-0.49, 0.0), 2.0);
    w.add(Shape::Circle { radius: 0.5 }, Vec2::new(0.49, 0.0), 1.0);
    w.bodies[0].velocity.x = 3.0;
    w.bodies[0].restitution = 1.0;
    w.bodies[1].restitution = 1.0;
    w.step(1.0 / 60.0);
    assert!((w.bodies[0].velocity.x * 2.0 + w.bodies[1].velocity.x - 6.0).abs() < 1e-5);
    assert!((w.energy() - 9.0).abs() < 0.001);
}
#[test]
fn rotated_boxes_have_two_contacts() {
    let mut a = Body::new(
        1,
        Shape::Box {
            half: Vec2::new(1.0, 0.5),
        },
        Vec2::ZERO,
        1.0,
    );
    a.angle = 0.3;
    let mut b = a.clone();
    b.id = 2;
    b.position = Vec2::new(0.0, 0.9).rotate(0.3);
    let c = collision::collide(&a, &b);
    assert_eq!(c.len(), 2);
    assert!(c.iter().all(|c| (c.penetration - 0.1).abs() < 0.001));
}
#[test]
fn stack_remains_finite_and_supported() {
    let mut w = World::new();
    w.add(
        Shape::Box {
            half: Vec2::new(5.0, 0.25),
        },
        Vec2::new(0.0, -0.25),
        0.0,
    );
    for i in 0..6 {
        w.add(
            Shape::Box {
                half: Vec2::new(0.5, 0.5),
            },
            Vec2::new(0.0, 0.5 + i as f32 * 1.01),
            1.0,
        );
    }
    run(&mut w, 10.0);
    for (i, b) in w.bodies.iter().enumerate().skip(1) {
        assert!(b.position.finite());
        assert!(
            (b.position.y - (i as f32 - 0.5)).abs() < 0.12,
            "body {i}: {:?}",
            b.position
        );
        assert!(b.velocity.length() < 0.3);
    }
}
#[test]
fn friction_slows_a_sliding_body() {
    let mut w = World::new();
    floor(&mut w);
    w.add(
        Shape::Box {
            half: Vec2::new(0.5, 0.5),
        },
        Vec2::new(0.0, 0.5),
        1.0,
    );
    w.bodies[1].velocity.x = 4.0;
    run(&mut w, 2.0);
    assert!(w.bodies[1].velocity.x.abs() < 0.2);
}
#[test]
fn joint_holds_its_length() {
    let mut w = World::new();
    let a = w.add(Shape::Circle { radius: 0.1 }, Vec2::new(0.0, 4.0), 0.0);
    let b = w.add(Shape::Circle { radius: 0.3 }, Vec2::new(1.0, 1.0), 1.0);
    w.add_joint(
        JointKind::Rod,
        a,
        b,
        Vec2::new(0.0, 4.0),
        Vec2::new(1.0, 1.0),
    );
    run(&mut w, 10.0);
    let (pa, pb) = w.joint_anchors(&w.joints[0]).unwrap();
    assert!(((pb - pa).length() - 10.0_f32.sqrt()).abs() < 0.03);
}
#[test]
fn scenes_survive_extended_simulation() {
    for scene in 0..SCENE_COUNT {
        let mut w = World::new();
        w.load_scene(scene);
        run(&mut w, 15.0);
        assert!(w
            .bodies
            .iter()
            .all(|b| b.position.finite() && b.velocity.finite() && b.angular_velocity.is_finite()));
        assert!(w.energy() < 100.0, "scene {scene} energy {}", w.energy());
        assert_eq!(w.culled, 0, "scene {scene} lost bodies");
    }
}
#[test]
fn deletion_cleans_up_constraints() {
    let mut w = World::new();
    w.load_scene(1);
    let id = w.joints[0].b;
    w.drag(id, w.bodies[2].position);
    w.remove(id);
    assert!(w.joints.iter().all(|j| j.a != id && j.b != id));
    assert!(w.mouse.is_none());
    w.step(1.0 / 60.0);
}
#[test]
fn invalid_input_does_not_poison_world() {
    let mut w = World::new();
    assert_eq!(
        w.add(Shape::Circle { radius: f32::NAN }, Vec2::ZERO, 1.0),
        0
    );
    let polygon = |sides| Shape::Polygon { radius: 1.0, sides };
    assert_eq!(w.add(polygon(2), Vec2::ZERO, 1.0), 0);
    assert_eq!(w.add(polygon(9), Vec2::ZERO, 1.0), 0);
    w.step(f32::NAN);
    assert_eq!(w.tick, 0);
    assert_eq!(w.time, 0.0);
}

#[test]
fn box_feature_ids_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for axis in 0..4 {
        for incident in 0..4 {
            for flip in [false, true] {
                for vertex in 0..4 {
                    assert!(seen.insert(box_feature(axis, incident, flip, vertex)));
                }
            }
        }
    }
}
#[test]
fn polygon_inertia_and_area_match_an_equivalent_box() {
    let square = Body::new(
        1,
        Shape::Polygon {
            radius: 2.0_f32.sqrt() * 0.5,
            sides: 4,
        },
        Vec2::ZERO,
        2.0,
    );
    let boxed = Body::new(
        2,
        Shape::Box {
            half: Vec2::new(0.5, 0.5),
        },
        Vec2::ZERO,
        2.0,
    );
    assert!((square.inv_inertia - boxed.inv_inertia).abs() < 1e-4);
    assert!((square.shape.area() - boxed.shape.area()).abs() < 1e-5);
    // The closing edge (last vertex to first) is the flat bottom, so the square is axis-aligned.
    let v0 = polygon_vertex(2.0_f32.sqrt() * 0.5, 4, 0);
    let v3 = polygon_vertex(2.0_f32.sqrt() * 0.5, 4, 3);
    assert!((v0 - Vec2::new(0.5, -0.5)).length() < 1e-5);
    assert!((v3 - Vec2::new(-0.5, -0.5)).length() < 1e-5);
}
#[test]
fn polygon_picking_and_bounds() {
    let mut w = World::new();
    let id = w.add(
        Shape::Polygon {
            radius: 1.0,
            sides: 3,
        },
        Vec2::new(2.0, 2.0),
        1.0,
    );
    assert_eq!(w.pick(Vec2::new(2.0, 2.0)), id);
    // Just outside the flat bottom edge (at y = 2 - 0.5).
    assert_eq!(w.pick(Vec2::new(2.0, 1.45)), 0);
    let (lo, hi) = collision::bounds(&w.bodies[0]);
    assert!((lo.y - 1.5).abs() < 1e-5 && (hi.y - 3.0).abs() < 1e-5);
    assert!((hi.x - lo.x - 3.0_f32.sqrt()).abs() < 1e-5);
}
#[test]
fn polygons_rest_flat_on_the_floor() {
    for sides in MIN_SIDES..=MAX_SIDES {
        let mut w = World::new();
        floor(&mut w);
        let radius = 0.5;
        let id = w.add(Shape::Polygon { radius, sides }, Vec2::new(0.0, 1.2), 1.0);
        run(&mut w, 4.0);
        let b = w.body(id).unwrap();
        let rest = radius * (std::f32::consts::PI / sides as f32).cos();
        assert!(
            (b.position.y - rest).abs() < 0.02,
            "{sides} sides at {:?}",
            b.position
        );
        assert!(b.velocity.length() < 0.05 && b.angle.abs() < 0.02);
    }
}
#[test]
fn polygon_circle_normals_point_from_a_to_b() {
    let hexagon = Body::new(
        1,
        Shape::Polygon {
            radius: 1.0,
            sides: 6,
        },
        Vec2::ZERO,
        0.0,
    );
    let ball = Body::new(2, Shape::Circle { radius: 0.5 }, Vec2::new(0.0, 1.3), 1.0);
    let down = collision::collide(&hexagon, &ball);
    assert_eq!(down.len(), 1);
    assert!((down[0].normal.y - 1.0).abs() < 1e-5);
    let top = 3.0_f32.sqrt() / 2.0;
    assert!((down[0].penetration - (top + 0.5 - 1.3)).abs() < 1e-5);
    let up = collision::collide(&ball, &hexagon);
    assert!((up[0].normal.y + 1.0).abs() < 1e-5);
    // Near a vertex the normal points from the corner to the circle center.
    let corner = Body::new(3, Shape::Circle { radius: 0.2 }, Vec2::new(1.1, 0.0), 1.0);
    let c = collision::collide(&hexagon, &corner);
    assert_eq!(c.len(), 1);
    assert!((c[0].normal.x - 1.0).abs() < 1e-5);
}
#[test]
fn mixed_shapes_stack_and_settle() {
    let mut w = World::new();
    floor(&mut w);
    let base = w.add(
        Shape::Polygon {
            radius: 0.6,
            sides: 6,
        },
        Vec2::new(0.0, 0.6),
        1.0,
    );
    let top = w.add(
        Shape::Box {
            half: Vec2::new(0.4, 0.3),
        },
        Vec2::new(0.0, 1.5),
        1.0,
    );
    run(&mut w, 5.0);
    let (b, t) = (w.body(base).unwrap(), w.body(top).unwrap());
    let hex_top = 0.6 * 2.0 * (std::f32::consts::PI / 6.0).cos();
    assert!(
        (t.position.y - (hex_top + 0.3)).abs() < 0.03,
        "{:?}",
        t.position
    );
    assert!(b.position.x.abs() < 0.05 && t.position.x.abs() < 0.05);
}
#[test]
fn sweep_matches_exhaustive_pairs() {
    let mut seed = 7u32;
    let mut rand = move || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    let mut w = World::new();
    for i in 0..200 {
        let p = Vec2::new(rand() * 30.0 - 15.0, rand() * 20.0);
        let shape = match i % 3 {
            0 => Shape::Circle {
                radius: 0.2 + rand(),
            },
            1 => Shape::Box {
                half: Vec2::new(0.1 + rand(), 0.1 + rand()),
            },
            _ => Shape::Polygon {
                radius: 0.2 + rand(),
                sides: 3 + i % 6,
            },
        };
        let mass = if i % 7 == 0 { 0.0 } else { 1.0 };
        let id = w.add(shape, p, mass);
        w.body_mut(id).unwrap().angle = rand() * 6.0;
    }
    let bounds: Vec<_> = w.bodies.iter().map(collision::bounds).collect();
    let mut expected = 0;
    for a in 0..w.bodies.len() {
        for b in a + 1..w.bodies.len() {
            let (amin, amax) = bounds[a];
            let (bmin, bmax) = bounds[b];
            let overlap =
                !(amax.x < bmin.x || bmax.x < amin.x || amax.y < bmin.y || bmax.y < amin.y);
            if overlap && (w.bodies[a].is_dynamic() || w.bodies[b].is_dynamic()) {
                expected += 1;
            }
        }
    }
    let pairs = w.broad_phase();
    assert_eq!(pairs.len(), expected);
    assert!(pairs.windows(2).all(|p| p[0] < p[1]));
}
#[test]
fn stacks_fall_asleep_and_wake_when_struck() {
    let mut w = World::new();
    floor(&mut w);
    let ids: Vec<u32> = (0..4)
        .map(|i| {
            w.add(
                Shape::Box {
                    half: Vec2::new(0.5, 0.5),
                },
                Vec2::new(0.0, 0.5 + i as f32 * 1.01),
                1.0,
            )
        })
        .collect();
    run(&mut w, 4.0);
    assert_eq!(w.sleeping_count(), 4, "stack should sleep");
    let resting: Vec<Vec2> = ids.iter().map(|&id| w.body(id).unwrap().position).collect();
    run(&mut w, 2.0);
    for (&id, p) in ids.iter().zip(&resting) {
        assert_eq!(
            w.body(id).unwrap().position,
            *p,
            "sleeping bodies must not drift"
        );
    }
    // A ball thrown at the top box wakes the whole island.
    let ball = w.add(Shape::Circle { radius: 0.3 }, Vec2::new(-3.0, 3.5), 2.0);
    w.body_mut(ball).unwrap().velocity = Vec2::new(12.0, 0.0);
    run(&mut w, 0.3);
    assert!(ids.iter().all(|&id| w.body(id).unwrap().awake));
    assert!(w.body(ids[3]).unwrap().position.x > 0.05);
}
#[test]
fn sleeping_can_be_disabled_and_impulses_wake() {
    let mut w = World::new();
    floor(&mut w);
    let id = w.add(Shape::Circle { radius: 0.5 }, Vec2::new(0.0, 0.5), 1.0);
    run(&mut w, 2.0);
    assert!(!w.body(id).unwrap().awake);
    w.apply_impulse(id, Vec2::new(0.0, 0.5), Vec2::new(2.0, 0.0));
    assert!(w.body(id).unwrap().awake);
    w.step(DT);
    assert!(w.body(id).unwrap().velocity.x > 1.0);
    w.configure(w.gravity, 12, false);
    run(&mut w, 6.0);
    assert!(w.body(id).unwrap().awake);
}
#[test]
fn removing_a_support_wakes_the_bodies_it_held() {
    let mut w = World::new();
    floor(&mut w);
    let shelf = w.add(
        Shape::Box {
            half: Vec2::new(1.0, 0.1),
        },
        Vec2::new(0.0, 2.0),
        0.0,
    );
    let ball = w.add(Shape::Circle { radius: 0.3 }, Vec2::new(0.0, 2.4), 1.0);
    run(&mut w, 2.0);
    assert!(!w.body(ball).unwrap().awake);
    w.remove(shelf);
    run(&mut w, 1.0);
    assert!(w.body(ball).unwrap().position.y < 0.5);
}
#[test]
fn rope_allows_slack_but_not_stretch() {
    let mut w = World::new();
    let ball = w.add(Shape::Circle { radius: 0.2 }, Vec2::new(0.0, 5.0), 1.0);
    let id = w.add_joint(
        JointKind::Rope,
        ball,
        0,
        Vec2::new(0.0, 5.0),
        Vec2::new(0.0, 8.0),
    );
    assert!(id > 0);
    // Throw the ball upward: a rope goes slack instead of pushing back.
    w.body_mut(ball).unwrap().velocity = Vec2::new(0.0, 5.0);
    let mut closest = f32::INFINITY;
    for _ in 0..240 {
        w.step(DT);
        let (pa, pb) = w.joint_anchors(&w.joints[0]).unwrap();
        closest = closest.min((pb - pa).length());
        assert!((pb - pa).length() < 3.0 + 0.05);
    }
    assert!(closest < 2.0, "rope never went slack: {closest}");
    let (pa, pb) = w.joint_anchors(&w.joints[0]).unwrap();
    assert!(((pb - pa).length() - 3.0).abs() < 0.05);
}
#[test]
fn spring_oscillates_at_its_frequency() {
    let mut w = World::new();
    w.gravity = Vec2::ZERO;
    w.allow_sleep = false;
    let ball = w.add(Shape::Circle { radius: 0.2 }, Vec2::new(1.5, 0.0), 1.0);
    w.add_joint(
        JointKind::Spring {
            frequency: 1.0,
            damping: 0.0,
        },
        ball,
        0,
        Vec2::new(1.5, 0.0),
        Vec2::ZERO,
    );
    w.joints[0].length = 1.0;
    let mut crossings = vec![];
    let mut last = 0.5;
    for i in 0..600 {
        w.step(DT);
        let x = w.bodies[0].position.x - 1.0;
        if last > 0.0 && x <= 0.0 {
            crossings.push(i as f32 * DT);
        }
        last = x;
    }
    assert!(crossings.len() >= 8);
    let period = (crossings[crossings.len() - 1] - crossings[0]) / (crossings.len() - 1) as f32;
    assert!((period - 1.0).abs() < 0.05, "period {period}");
    assert!(w.potential_energy() > 0.0 || w.energy() > 0.0);
}
#[test]
fn pins_hold_anchors_together_and_motors_spin() {
    let mut w = World::new();
    w.allow_sleep = false;
    // An off-center pin to the world makes a box swing like a pendulum.
    let swing = w.add(
        Shape::Box {
            half: Vec2::new(1.0, 0.2),
        },
        Vec2::new(0.0, 5.0),
        1.0,
    );
    let pin_at = Vec2::new(-0.9, 5.0);
    let pin = JointKind::Pin {
        motor_speed: 0.0,
        max_torque: 0.0,
    };
    assert!(w.add_joint(pin, swing, 0, pin_at, pin_at) > 0);
    let mut max_gap: f32 = 0.0;
    let mut lowest = f32::INFINITY;
    for _ in 0..300 {
        w.step(DT);
        let (pa, pb) = w.joint_anchors(&w.joints[0]).unwrap();
        max_gap = max_gap.max((pb - pa).length());
        lowest = lowest.min(w.body(swing).unwrap().position.y);
    }
    assert!(max_gap < 0.03, "pin separated by {max_gap}");
    // At the bottom of the swing the center hangs 0.9 m below the pin.
    assert!((lowest - 4.1).abs() < 0.05, "lowest {lowest}");
    // A motor brings a free wheel up to speed.
    let mut w = World::new();
    w.gravity = Vec2::ZERO;
    let wheel = w.add(Shape::Circle { radius: 0.5 }, Vec2::ZERO, 1.0);
    w.add_joint(
        JointKind::Pin {
            motor_speed: 3.0,
            max_torque: 50.0,
        },
        wheel,
        0,
        Vec2::ZERO,
        Vec2::ZERO,
    );
    run(&mut w, 1.0);
    let b = w.body(wheel).unwrap();
    assert!((b.angular_velocity - 3.0).abs() < 0.01);
    assert!(b.position.length() < 1e-3);
    assert!(b.awake, "motor-driven bodies never sleep");
}
#[test]
fn joint_validation_rejects_impossible_links() {
    let mut w = World::new();
    let s1 = w.add(Shape::Circle { radius: 0.2 }, Vec2::ZERO, 0.0);
    let s2 = w.add(Shape::Circle { radius: 0.2 }, Vec2::new(1.0, 0.0), 0.0);
    let d = w.add(Shape::Circle { radius: 0.2 }, Vec2::new(2.0, 0.0), 1.0);
    let rod = JointKind::Rod;
    assert_eq!(w.add_joint(rod, s1, s2, Vec2::ZERO, Vec2::new(1.0, 0.0)), 0);
    assert_eq!(w.add_joint(rod, s1, 0, Vec2::ZERO, Vec2::new(1.0, 0.0)), 0);
    assert_eq!(w.add_joint(rod, d, d, Vec2::ZERO, Vec2::new(1.0, 0.0)), 0);
    assert_eq!(w.add_joint(rod, d, 99, Vec2::ZERO, Vec2::new(1.0, 0.0)), 0);
    assert_eq!(
        w.add_joint(rod, d, s1, Vec2::new(2.0, 0.0), Vec2::new(2.0, 0.0)),
        0
    );
    // World-first arguments are normalized so `a` is always a body.
    let id = w.add_joint(rod, 0, d, Vec2::new(2.0, 3.0), Vec2::new(2.0, 0.0));
    assert!(id > 0);
    let j = w.joints.iter().find(|j| j.id == id).unwrap();
    assert_eq!((j.a, j.b), (d, 0));
    assert_eq!(j.local_b, Vec2::new(2.0, 3.0));
    w.remove_joint(id);
    assert!(w.joints.is_empty());
}
#[test]
fn ids_can_be_chosen_and_stay_sorted() {
    let mut w = World::new();
    let circle = Shape::Circle { radius: 0.2 };
    assert_eq!(w.add_with_id(10, circle, Vec2::ZERO, 1.0), 10);
    assert_eq!(w.add_with_id(5, circle, Vec2::ZERO, 1.0), 5);
    assert_eq!(w.add_with_id(5, circle, Vec2::ZERO, 1.0), 0);
    assert_eq!(w.add(circle, Vec2::ZERO, 1.0), 11);
    let ids: Vec<u32> = w.bodies.iter().map(|b| b.id).collect();
    assert_eq!(ids, vec![5, 10, 11]);
    assert_eq!(w.body(10).unwrap().id, 10);
}
#[test]
fn history_rewinds_and_branches() {
    let mut w = World::new();
    w.load_scene(3);
    let mut h = History::new(600);
    h.sync(&w);
    let mut states = vec![];
    for _ in 0..30 {
        w.step(DT);
        h.record(&w);
        states.push((w.tick, w.bodies[1].position));
    }
    assert_eq!(h.len(), 31);
    assert!(h.seek(&mut w, 10));
    assert_eq!(w.tick, 10);
    assert_eq!(w.bodies[1].position, states[9].1);
    // Scrubbing forward again replays the recorded future exactly.
    assert!(h.seek(&mut w, 30));
    assert_eq!(w.bodies[1].position, states[29].1);
    // Stepping from a rewound state discards the old future.
    h.seek(&mut w, 10);
    w.step(DT);
    h.record(&w);
    assert_eq!(h.len(), 12);
    assert_eq!(h.cursor(), 11);
    assert_eq!(w.tick, 11);
    // Structural edits start a new timeline.
    w.add(Shape::Circle { radius: 0.2 }, Vec2::new(0.0, 5.0), 1.0);
    h.sync(&w);
    assert_eq!(h.len(), 1);
    assert!(!h.seek(&mut w, 5));
    // Reloading a scene never reuses a timeline, even with an identical structure.
    w.load_scene(3);
    for _ in 0..5 {
        w.step(DT);
        h.record(&w);
    }
    let version = w.structure_version();
    w.load_scene(3);
    assert!(w.structure_version() > version);
    h.sync(&w);
    assert_eq!(h.len(), 1);
}
#[test]
fn history_is_bounded() {
    let mut w = World::new();
    w.load_scene(0);
    let mut h = History::new(50);
    for _ in 0..80 {
        w.step(DT);
        h.record(&w);
    }
    assert_eq!(h.len(), 50);
    assert_eq!(h.cursor(), 49);
    assert!(h.seek(&mut w, 0));
    assert_eq!(w.tick, 31);
}
#[test]
fn escaped_bodies_are_culled_with_their_joints() {
    let mut w = World::new();
    let a = w.add(Shape::Circle { radius: 0.2 }, Vec2::new(0.0, 0.0), 1.0);
    let b = w.add(Shape::Circle { radius: 0.2 }, Vec2::new(1.0, 0.0), 1.0);
    w.add_joint(
        JointKind::Rope,
        a,
        b,
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
    );
    run(&mut w, 4.0);
    assert!(w.bodies.is_empty());
    assert!(w.joints.is_empty());
    assert_eq!(w.culled, 2);
}
#[test]
fn speeds_are_capped() {
    let mut w = World::new();
    w.gravity = Vec2::ZERO;
    let id = w.add(Shape::Circle { radius: 0.2 }, Vec2::ZERO, 1.0);
    w.apply_impulse(id, Vec2::new(0.0, 0.2), Vec2::new(1e6, 0.0));
    w.step(DT);
    let b = w.body(id).unwrap();
    assert!(b.velocity.length() <= MAX_SPEED + 1e-3);
    assert!(b.angular_velocity.abs() <= MAX_ANGULAR_SPEED);
}
#[test]
fn potential_energy_counts_height_and_springs() {
    let mut w = World::new();
    let id = w.add(Shape::Circle { radius: 0.2 }, Vec2::new(0.0, 2.0), 3.0);
    assert!((w.potential_energy() - 3.0 * 9.81 * 2.0).abs() < 1e-3);
    w.gravity = Vec2::ZERO;
    w.add_joint(
        JointKind::Spring {
            frequency: 1.0,
            damping: 0.0,
        },
        id,
        0,
        Vec2::new(0.0, 2.0),
        Vec2::new(0.0, 0.0),
    );
    w.joints[0].length = 1.0;
    let stiffness = 3.0 * std::f32::consts::TAU.powi(2);
    assert!((w.potential_energy() - 0.5 * stiffness).abs() < 1e-2);
}
#[test]
fn removal_keeps_remaining_contacts_consistent() {
    let mut w = World::new();
    w.load_scene(0);
    run(&mut w, 0.5);
    let before = w.contacts.len();
    let victim = w.bodies[5].id;
    w.remove(victim);
    assert!(w.contacts.len() < before);
    for c in &w.contacts {
        assert!(c.a < c.b && c.b < w.bodies.len());
        // Every remaining contact still describes its own pair.
        let fresh = collision::collide(&w.bodies[c.a], &w.bodies[c.b]);
        assert!(fresh.iter().any(|g| g.feature == c.feature));
    }
    w.step(DT);
}
#[test]
fn dominoes_topple_in_sequence() {
    let mut w = World::new();
    w.load_scene(4);
    run(&mut w, 10.0);
    let fallen = w
        .bodies
        .iter()
        .filter(|b| matches!(b.shape, Shape::Box { half } if half.x < 0.1))
        .filter(|b| b.angle.abs() > 0.4)
        .count();
    assert_eq!(fallen, 18, "every domino should fall");
}
#[test]
fn placement_and_property_edits_wake_bodies() {
    let mut w = World::new();
    floor(&mut w);
    let id = w.add(Shape::Circle { radius: 0.4 }, Vec2::new(0.0, 0.4), 1.0);
    run(&mut w, 2.0);
    assert!(!w.body(id).unwrap().awake);
    w.place(id, Vec2::new(2.0, 3.0));
    assert!(w.body(id).unwrap().awake);
    run(&mut w, 1.5);
    assert!(w.body(id).unwrap().position.y < 0.5);
    w.set_properties(id, 0.0, 0.5, 0.5);
    assert!(!w.body(id).unwrap().is_dynamic());
    w.set_properties(id, 2.0, 0.5, 0.5);
    assert!(w.body(id).unwrap().awake);
}
