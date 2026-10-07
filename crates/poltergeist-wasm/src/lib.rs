//! Thin scalar ABI: the browser owns no Rust pointers and copies each packed frame.
//! Layout is versioned in web/src/engine.ts. No third-party crates are needed.
use poltergeist_core::{Shape, Vec2, World};
use std::cell::RefCell;

struct Engine {
    world: World,
    frame: Vec<f32>,
}
thread_local! {
    static ENGINE: RefCell<Engine> = RefCell::new(Engine {world:World::new(),frame:Vec::new()});
}
fn with_world<T>(f: impl FnOnce(&mut World) -> T) -> T {
    ENGINE.with(|e| f(&mut e.borrow_mut().world))
}

#[no_mangle]
pub extern "C" fn reset(scene: u32) {
    with_world(|w| w.load_scene(scene));
}
#[no_mangle]
pub extern "C" fn clear() {
    with_world(|w| *w = World::new());
}
#[no_mangle]
pub extern "C" fn step(dt: f32) {
    with_world(|w| w.step(dt));
}
#[no_mangle]
pub extern "C" fn configure(gravity: f32, iterations: u32) {
    if gravity.is_finite() {
        with_world(|w| {
            w.gravity = Vec2::new(0.0, -gravity.clamp(0.0, 30.0));
            w.iterations = iterations.clamp(1, 32) as usize;
        });
    }
}
#[no_mangle]
pub extern "C" fn spawn(kind: u32, x: f32, y: f32, a: f32, b: f32, mass: f32) -> u32 {
    let shape = if kind == 0 {
        Shape::Circle { radius: a }
    } else {
        Shape::Box {
            half: Vec2::new(a, b),
        }
    };
    with_world(|w| w.add(shape, Vec2::new(x, y), mass))
}
#[no_mangle]
pub extern "C" fn motion(id: u32, angle: f32, vx: f32, vy: f32, omega: f32) {
    if [angle, vx, vy, omega].iter().all(|v| v.is_finite()) {
        with_world(|w| {
            if let Some(b) = w.body_mut(id) {
                b.angle = angle;
                b.velocity = Vec2::new(vx, vy);
                b.angular_velocity = omega;
            }
        });
    }
}
#[no_mangle]
pub extern "C" fn properties(id: u32, mass: f32, friction: f32, restitution: f32) {
    if [mass, friction, restitution].iter().all(|v| v.is_finite()) {
        with_world(|w| {
            if let Some(b) = w.body_mut(id) {
                b.set_mass(mass.clamp(0.0, 100.0));
                b.friction = friction.clamp(0.0, 1.5);
                b.restitution = restitution.clamp(0.0, 1.0);
                if b.inv_mass == 0.0 {
                    b.velocity = Vec2::ZERO;
                    b.angular_velocity = 0.0;
                }
            }
        });
    }
}
#[no_mangle]
pub extern "C" fn remove(id: u32) {
    with_world(|w| w.remove(id));
}
#[no_mangle]
pub extern "C" fn pick(x: f32, y: f32) -> u32 {
    with_world(|w| w.pick(Vec2::new(x, y)))
}
#[no_mangle]
pub extern "C" fn impulse(id: u32, x: f32, y: f32, jx: f32, jy: f32) {
    with_world(|w| w.apply_impulse(id, Vec2::new(x, y), Vec2::new(jx, jy)));
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
#[no_mangle]
pub extern "C" fn joint(a: u32, b: u32, ax: f32, ay: f32, bx: f32, by: f32, length: f32) {
    if [ax, ay, bx, by, length].iter().all(|v| v.is_finite()) && a != b && length > 0.0 {
        with_world(|w| {
            let count = w.joints.len();
            if count >= 256 {
                return;
            }
            w.add_joint(a, b, Vec2::new(ax, ay), Vec2::new(bx, by));
            if w.joints.len() > count {
                w.joints.last_mut().unwrap().length = length;
            }
        });
    }
}
#[no_mangle]
pub extern "C" fn frame_ptr() -> *const f32 {
    ENGINE.with(|e| {
        let mut e = e.borrow_mut();
        let Engine { world: w, frame: f } = &mut *e;
        f.clear();
        let depth = w.contacts.iter().map(|c| c.penetration).fold(0.0, f32::max);
        f.extend_from_slice(&[
            1.0,
            w.bodies.len() as f32,
            w.contacts.len() as f32,
            w.joints.len() as f32,
            w.time,
            w.tick as f32,
            w.candidate_pairs as f32,
            depth,
            w.energy(),
        ]);
        for b in &w.bodies {
            let (kind, a, bsize) = match b.shape {
                Shape::Circle { radius } => (0.0, radius, radius),
                Shape::Box { half } => (1.0, half.x, half.y),
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
        for j in &w.joints {
            if let Some((a, b)) = w.joint_anchors(j) {
                f.extend_from_slice(&[
                    a.x, a.y, b.x, b.y, j.length, j.impulse, j.a as f32, j.b as f32,
                ]);
            }
        }
        f.as_ptr()
    })
}
#[no_mangle]
pub extern "C" fn frame_len() -> usize {
    ENGINE.with(|e| e.borrow().frame.len())
}
