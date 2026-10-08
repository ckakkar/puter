//! Thin scalar ABI: the browser owns no Rust pointers and copies each packed frame.
//! Layout is versioned in web/src/engine.ts. No third-party crates are needed.
use poltergeist_core::history::History;
use poltergeist_core::{JointKind, Shape, Vec2, World, MAX_SIDES};
use std::cell::RefCell;

/// Ten seconds of ticks at 60 Hz.
const HISTORY_TICKS: usize = 600;
const FRAME_VERSION: f32 = 2.0;

struct Engine {
    world: World,
    history: History,
    frame: Vec<f32>,
}
thread_local! {
    static ENGINE: RefCell<Engine> = RefCell::new(Engine {
        world: World::new(),
        history: History::new(HISTORY_TICKS),
        frame: Vec::new(),
    });
}
fn with_world<T>(f: impl FnOnce(&mut World) -> T) -> T {
    ENGINE.with(|e| f(&mut e.borrow_mut().world))
}
/// Runs an edit, then re-anchors the timeline so rewinding reflects it.
fn edit<T>(f: impl FnOnce(&mut World) -> T) -> T {
    ENGINE.with(|e| {
        let e = &mut *e.borrow_mut();
        let result = f(&mut e.world);
        e.history.sync(&e.world);
        result
    })
}
fn shape(kind: u32, a: f32, b: f32) -> Option<Shape> {
    match kind {
        0 => Some(Shape::Circle { radius: a }),
        1 => Some(Shape::Box {
            half: Vec2::new(a, b),
        }),
        2 if b.is_finite() && b >= 0.0 && b <= MAX_SIDES as f32 && b.fract() == 0.0 => {
            Some(Shape::Polygon {
                radius: a,
                sides: b as u32,
            })
        }
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn reset(scene: u32) {
    edit(|w| w.load_scene(scene));
}
#[no_mangle]
pub extern "C" fn clear() {
    edit(|w| w.clear());
}
/// Recomputes displayed contacts without advancing time.
#[no_mangle]
pub extern "C" fn refresh() {
    with_world(|w| w.refresh_contacts());
}
#[no_mangle]
pub extern "C" fn step(dt: f32) {
    ENGINE.with(|e| {
        let e = &mut *e.borrow_mut();
        e.world.step(dt);
        e.history.record(&e.world);
    });
}
#[no_mangle]
pub extern "C" fn configure(gravity: f32, iterations: u32, sleeping: u32) {
    if gravity.is_finite() {
        edit(|w| {
            w.configure(
                Vec2::new(0.0, -gravity.clamp(0.0, 30.0)),
                iterations.clamp(1, 32) as usize,
                sleeping != 0,
            )
        });
    }
}
/// Adds a body with the requested ID (0 for automatic). Returns its ID, or 0 on failure.
#[no_mangle]
pub extern "C" fn spawn(id: u32, kind: u32, x: f32, y: f32, a: f32, b: f32, mass: f32) -> u32 {
    match shape(kind, a, b) {
        Some(shape) => edit(|w| w.add_with_id(id, shape, Vec2::new(x, y), mass)),
        None => 0,
    }
}
#[no_mangle]
pub extern "C" fn motion(id: u32, angle: f32, vx: f32, vy: f32, omega: f32) {
    edit(|w| w.set_motion(id, angle, Vec2::new(vx, vy), omega));
}
#[no_mangle]
pub extern "C" fn place(id: u32, x: f32, y: f32) {
    edit(|w| w.place(id, Vec2::new(x, y)));
}
#[no_mangle]
pub extern "C" fn properties(id: u32, mass: f32, friction: f32, restitution: f32) {
    edit(|w| w.set_properties(id, mass, friction, restitution));
}
#[no_mangle]
pub extern "C" fn remove(id: u32) {
    edit(|w| w.remove(id));
}
#[no_mangle]
pub extern "C" fn pick(x: f32, y: f32) -> u32 {
    with_world(|w| w.pick(Vec2::new(x, y)))
}
/// The topmost body under a point other than `exclude`; used to choose what a pin attaches to.
#[no_mangle]
pub extern "C" fn pick_below(x: f32, y: f32, exclude: u32) -> u32 {
    with_world(|w| w.pick_excluding(Vec2::new(x, y), exclude))
}
#[no_mangle]
pub extern "C" fn impulse(id: u32, x: f32, y: f32, jx: f32, jy: f32) {
    edit(|w| w.apply_impulse(id, Vec2::new(x, y), Vec2::new(jx, jy)));
}
#[no_mangle]
pub extern "C" fn drag_start(id: u32, x: f32, y: f32) {
    with_world(|w| w.drag(id, Vec2::new(x, y)));
}
#[no_mangle]
pub extern "C" fn drag_to(x: f32, y: f32) {
    if x.is_finite() && y.is_finite() {
        with_world(|w| {
            if let Some(m) = &mut w.mouse {
                m.target = Vec2::new(x, y);
            }
        });
    }
}
#[no_mangle]
pub extern "C" fn drag_end() {
    with_world(|w| w.mouse = None);
}
/// Connects body `a` to body `b` (0 = the world). With `local` set, anchors are
/// body-local (as stored in scenes) and `length` is used as given; otherwise anchors
/// are world points and distance joints measure their length from them.
/// Returns the joint ID, or 0 if the joint is invalid.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn joint(
    id: u32,
    kind: u32,
    a: u32,
    b: u32,
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    length: f32,
    p1: f32,
    p2: f32,
    local: u32,
) -> u32 {
    let Some(kind) = JointKind::from_code(kind, p1, p2) else {
        return 0;
    };
    let (pa, pb) = (Vec2::new(ax, ay), Vec2::new(bx, by));
    edit(|w| {
        if local != 0 {
            w.add_joint_local(id, kind, a, b, pa, pb, length)
        } else {
            let created = w.add_joint(kind, a, b, pa, pb);
            if created != 0 && length.is_finite() && length > 0.0 {
                w.set_joint(created, length, p1, p2);
            }
            created
        }
    })
}
#[no_mangle]
pub extern "C" fn joint_set(id: u32, length: f32, p1: f32, p2: f32) {
    edit(|w| w.set_joint(id, length, p1, p2));
}
#[no_mangle]
pub extern "C" fn joint_remove(id: u32) {
    edit(|w| w.remove_joint(id));
}
/// Sets the simulation clock, e.g. when restoring an undo checkpoint.
#[no_mangle]
pub extern "C" fn clock(time: f32, tick: u32) {
    if time.is_finite() && time >= 0.0 {
        edit(|w| {
            w.time = time;
            w.tick = tick as u64;
        });
    }
}
/// Restores the recorded state at `index` (0 is the oldest still kept).
#[no_mangle]
pub extern "C" fn history_seek(index: u32) -> u32 {
    ENGINE.with(|e| {
        let e = &mut *e.borrow_mut();
        e.history.seek(&mut e.world, index as usize) as u32
    })
}
#[no_mangle]
pub extern "C" fn frame_ptr() -> *const f32 {
    ENGINE.with(|e| {
        let mut e = e.borrow_mut();
        let Engine {
            world: w,
            history: h,
            frame: f,
        } = &mut *e;
        f.clear();
        let depth = w.contacts.iter().map(|c| c.penetration).fold(0.0, f32::max);
        let joints: Vec<_> = w
            .joints
            .iter()
            .filter_map(|j| w.joint_anchors(j).map(|anchors| (j, anchors)))
            .collect();
        f.extend_from_slice(&[
            FRAME_VERSION,
            w.bodies.len() as f32,
            w.contacts.len() as f32,
            joints.len() as f32,
            w.time,
            w.tick as f32,
            w.candidate_pairs as f32,
            depth,
            w.energy(),
            w.potential_energy(),
            w.sleeping_count() as f32,
            w.culled as f32,
            h.len() as f32,
            h.cursor() as f32,
        ]);
        for b in &w.bodies {
            let (kind, a, bsize) = match b.shape {
                Shape::Circle { radius } => (0.0, radius, radius),
                Shape::Box { half } => (1.0, half.x, half.y),
                Shape::Polygon { radius, sides } => (2.0, radius, sides as f32),
            };
            f.extend_from_slice(&[
                b.id as f32,
                kind,
                b.position.x,
                b.position.y,
                b.angle,
                a,
                bsize,
                b.mass,
                b.friction,
                b.restitution,
                b.velocity.x,
                b.velocity.y,
                b.angular_velocity,
                (b.awake || !b.is_dynamic()) as u32 as f32,
            ]);
        }
        for c in &w.contacts {
            f.extend_from_slice(&[
                c.point.x,
                c.point.y,
                c.normal.x,
                c.normal.y,
                c.penetration,
                c.normal_impulse,
                c.tangent_impulse,
                w.bodies[c.a].id as f32,
                w.bodies[c.b].id as f32,
            ]);
        }
        for (j, (a, b)) in joints {
            let (p1, p2) = j.kind.params();
            f.extend_from_slice(&[
                j.id as f32,
                j.kind.code() as f32,
                a.x,
                a.y,
                b.x,
                b.y,
                j.local_a.x,
                j.local_a.y,
                j.local_b.x,
                j.local_b.y,
                j.length,
                j.impulse,
                j.a as f32,
                j.b as f32,
                p1,
                p2,
            ]);
        }
        f.as_ptr()
    })
}
#[no_mangle]
pub extern "C" fn frame_len() -> usize {
    ENGINE.with(|e| e.borrow().frame.len())
}
