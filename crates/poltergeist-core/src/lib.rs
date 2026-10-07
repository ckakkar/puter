//! Small inspectable 2D rigid-body engine. Units: meters, kilograms, seconds, radians.
//! Single-threaded, discrete collisions; stable ordering, no cross-platform bit-exact promise.
pub mod collision;
pub mod math;
use math::angular_velocity;
pub use math::Vec2;
use std::collections::BTreeMap;

pub const MAX_BODIES: usize = 256;
#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Circle { radius: f32 },
    Box { half: Vec2 },
}
#[derive(Clone, Debug)]
pub struct Body {
    pub id: u32,
    pub shape: Shape,
    pub position: Vec2,
    pub angle: f32,
    pub velocity: Vec2,
    pub angular_velocity: f32,
    pub mass: f32,
    pub inv_mass: f32,
    pub inv_inertia: f32,
    pub friction: f32,
    pub restitution: f32,
}
impl Body {
    pub fn new(id: u32, shape: Shape, position: Vec2, mass: f32) -> Self {
        let mut body = Self {
            id,
            shape,
            position,
            angle: 0.0,
            velocity: Vec2::ZERO,
            angular_velocity: 0.0,
            mass: 0.0,
            inv_mass: 0.0,
            inv_inertia: 0.0,
            friction: 0.55,
            restitution: 0.12,
        };
        body.set_mass(mass);
        body
    }
    pub fn set_mass(&mut self, mass: f32) {
        self.mass = mass.max(0.0);
        self.inv_mass = if self.mass > 0.0 {
            1.0 / self.mass
        } else {
            0.0
        };
        let inertia = match self.shape {
            Shape::Circle { radius } => 0.5 * self.mass * radius * radius,
            Shape::Box { half } => self.mass * (half.x * half.x + half.y * half.y) / 3.0,
        };
        self.inv_inertia = if inertia > 0.0 { 1.0 / inertia } else { 0.0 };
    }
    pub fn point_velocity(&self, p: Vec2) -> Vec2 {
        self.velocity + angular_velocity(self.angular_velocity, p - self.position)
    }
    pub fn impulse(&mut self, p: Vec2, j: Vec2) {
        self.velocity += j * self.inv_mass;
        self.angular_velocity += (p - self.position).cross(j) * self.inv_inertia;
    }
}
#[derive(Clone, Debug)]
pub struct Contact {
    pub a: usize,
    pub b: usize,
    pub point: Vec2,
    pub normal: Vec2,
    pub penetration: f32,
    pub normal_impulse: f32,
    pub tangent_impulse: f32,
    pub feature: u32,
    target_velocity: f32,
}
#[derive(Clone, Debug)]
pub struct DistanceJoint {
    pub a: u32,
    pub b: u32,
    pub local_a: Vec2,
    pub local_b: Vec2,
    pub length: f32,
    pub impulse: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct MouseJoint {
    pub id: u32,
    pub local_anchor: Vec2,
    pub target: Vec2,
}
#[derive(Clone, Copy)]
struct Cached {
    normal: Vec2,
    normal_impulse: f32,
    tangent_impulse: f32,
}

pub struct World {
    pub bodies: Vec<Body>,
    pub contacts: Vec<Contact>,
    pub joints: Vec<DistanceJoint>,
    pub gravity: Vec2,
    pub iterations: usize,
    pub time: f32,
    pub tick: u64,
    pub candidate_pairs: usize,
    pub mouse: Option<MouseJoint>,
    next_id: u32,
    cache: BTreeMap<(u32, u32, u32), Cached>,
}
impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
impl World {
    pub fn new() -> Self {
        Self {
            bodies: vec![],
            contacts: vec![],
            joints: vec![],
            gravity: Vec2::new(0.0, -9.81),
            iterations: 12,
            time: 0.0,
            tick: 0,
            candidate_pairs: 0,
            mouse: None,
            next_id: 1,
            cache: BTreeMap::new(),
        }
    }
    pub fn add(&mut self, shape: Shape, p: Vec2, mass: f32) -> u32 {
        if self.bodies.len() >= MAX_BODIES || !p.finite() || !mass.is_finite() || mass < 0.0 {
            return 0;
        }
        match shape {
            Shape::Circle { radius } if !radius.is_finite() || radius <= 0.0 => return 0,
            Shape::Box { half } if !half.finite() || half.x <= 0.0 || half.y <= 0.0 => return 0,
            _ => {}
        }
        let id = self.next_id;
        self.next_id += 1;
        self.bodies.push(Body::new(id, shape, p, mass));
        id
    }
    pub fn body_mut(&mut self, id: u32) -> Option<&mut Body> {
        self.bodies.iter_mut().find(|b| b.id == id)
    }
    pub fn remove(&mut self, id: u32) {
        self.bodies.retain(|b| b.id != id);
        self.joints.retain(|j| j.a != id && j.b != id);
        self.contacts.clear();
        self.cache.clear();
        if self.mouse.is_some_and(|m| m.id == id) {
            self.mouse = None;
        }
    }
    pub fn pick(&self, p: Vec2) -> u32 {
        self.bodies
            .iter()
            .rev()
            .find(|b| {
                let q = (p - b.position).rotate(-b.angle);
                match b.shape {
                    Shape::Circle { radius } => q.dot(q) <= radius * radius,
                    Shape::Box { half } => q.x.abs() <= half.x && q.y.abs() <= half.y,
                }
            })
            .map_or(0, |b| b.id)
    }
    pub fn apply_impulse(&mut self, id: u32, p: Vec2, j: Vec2) {
        if p.finite() && j.finite() {
            if let Some(b) = self.body_mut(id) {
                b.impulse(p, j);
            }
        }
    }
    pub fn drag(&mut self, id: u32, p: Vec2) {
        if !p.finite() {
            return;
        }
        if let Some(b) = self.bodies.iter().find(|b| b.id == id && b.inv_mass > 0.0) {
            self.mouse = Some(MouseJoint {
                id,
                local_anchor: (p - b.position).rotate(-b.angle),
                target: p,
            });
        }
    }
    pub fn add_joint(&mut self, a: u32, b: u32, pa: Vec2, pb: Vec2) {
        let Some(ba) = self.bodies.iter().find(|x| x.id == a) else {
            return;
        };
        let Some(bb) = self.bodies.iter().find(|x| x.id == b) else {
            return;
        };
        self.joints.push(DistanceJoint {
            a,
            b,
            local_a: (pa - ba.position).rotate(-ba.angle),
            local_b: (pb - bb.position).rotate(-bb.angle),
            length: (pb - pa).length(),
            impulse: 0.0,
        });
    }
    pub fn joint_anchors(&self, j: &DistanceJoint) -> Option<(Vec2, Vec2)> {
        let a = self.bodies.iter().find(|b| b.id == j.a)?;
        let b = self.bodies.iter().find(|b| b.id == j.b)?;
        Some((
            a.position + j.local_a.rotate(a.angle),
            b.position + j.local_b.rotate(b.angle),
        ))
    }
    fn detect(&mut self) -> Vec<Contact> {
        let mut contacts = vec![];
        self.candidate_pairs = 0;
        let bounds: Vec<_> = self.bodies.iter().map(collision::bounds).collect();
        for a in 0..self.bodies.len() {
            for b in a + 1..self.bodies.len() {
                let ba = &self.bodies[a];
                let bb = &self.bodies[b];
                if ba.inv_mass == 0.0 && bb.inv_mass == 0.0 {
                    continue;
                }
                let (amin, amax) = bounds[a];
                let (bmin, bmax) = bounds[b];
                if amax.x < bmin.x || bmax.x < amin.x || amax.y < bmin.y || bmax.y < amin.y {
                    continue;
                }
                self.candidate_pairs += 1;
                for g in collision::collide(ba, bb) {
                    let cached = self
                        .cache
                        .get(&(ba.id, bb.id, g.feature))
                        .filter(|c| c.normal.dot(g.normal) > 0.95);
                    let vn =
                        (bb.point_velocity(g.point) - ba.point_velocity(g.point)).dot(g.normal);
                    contacts.push(Contact {
                        a,
                        b,
                        point: g.point,
                        normal: g.normal,
                        penetration: g.penetration,
                        feature: g.feature,
                        normal_impulse: cached.map_or(0.0, |c| c.normal_impulse),
                        tangent_impulse: cached.map_or(0.0, |c| c.tangent_impulse),
                        target_velocity: if vn < -1.0 {
                            -ba.restitution.min(bb.restitution) * vn
                        } else {
                            0.0
                        },
                    });
                }
            }
        }
        contacts
    }
    pub fn step(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 || dt > 0.1 {
            return;
        }
        // Two substeps keep this compact discrete solver useful at normal sandbox speeds.
        for _ in 0..2 {
            self.substep(dt * 0.5);
        }
        self.time += dt;
        self.tick += 1;
    }
    fn substep(&mut self, dt: f32) {
        for b in &mut self.bodies {
            if b.inv_mass > 0.0 {
                b.velocity += self.gravity * dt;
            }
        }
        if let Some(mouse) = self.mouse {
            if let Some(b) = self.body_mut(mouse.id) {
                let p = b.position + mouse.local_anchor.rotate(b.angle);
                let spring = (mouse.target - p) * 55.0 - b.point_velocity(p) * 10.0;
                // Cap acceleration so distant pointer jumps cannot launch bodies arbitrarily fast.
                let force = spring * (80.0 / spring.length().max(80.0));
                b.impulse(p, force * (b.mass * dt));
            }
        }
        let mut contacts = self.detect();
        for c in &contacts {
            let impulse = c.normal * c.normal_impulse + c.normal.perp() * c.tangent_impulse;
            self.bodies[c.a].impulse(c.point, -impulse);
            self.bodies[c.b].impulse(c.point, impulse);
        }
        for joint in &mut self.joints {
            joint.impulse = 0.0;
        }
        for _ in 0..self.iterations.clamp(1, 32) {
            for c in &mut contacts {
                solve_contact(&mut self.bodies, c);
            }
            for i in 0..self.joints.len() {
                self.solve_joint(i, dt);
            }
        }
        for b in &mut self.bodies {
            if b.inv_mass > 0.0 {
                b.position += b.velocity * dt;
                b.angle += b.angular_velocity * dt;
            }
        }
        // Separate positional projection limits energy introduced by overlap correction.
        for _ in 0..4 {
            for c in &contacts {
                let fresh = collision::collide(&self.bodies[c.a], &self.bodies[c.b]);
                for g in fresh {
                    let a = &self.bodies[c.a];
                    let b = &self.bodies[c.b];
                    let ra = g.point - a.position;
                    let rb = g.point - b.position;
                    let k = a.inv_mass
                        + b.inv_mass
                        + a.inv_inertia * ra.cross(g.normal).powi(2)
                        + b.inv_inertia * rb.cross(g.normal).powi(2);
                    if k <= 0.0 {
                        continue;
                    }
                    let magnitude = (0.25 * (g.penetration - 0.004).max(0.0)).min(0.08) / k;
                    let p = g.normal * magnitude;
                    let am = a.inv_mass;
                    self.bodies[c.a].position -= p * am;
                    // Borrow values before mutating their owners.
                    let ai = self.bodies[c.a].inv_inertia;
                    let bi = self.bodies[c.b].inv_inertia;
                    let bm = self.bodies[c.b].inv_mass;
                    self.bodies[c.a].angle -= ra.cross(p) * ai;
                    self.bodies[c.b].position += p * bm;
                    self.bodies[c.b].angle += rb.cross(p) * bi;
                }
            }
        }
        self.cache.clear();
        for c in &contacts {
            self.cache.insert(
                (self.bodies[c.a].id, self.bodies[c.b].id, c.feature),
                Cached {
                    normal: c.normal,
                    normal_impulse: c.normal_impulse,
                    tangent_impulse: c.tangent_impulse,
                },
            );
        }
        self.contacts = contacts;
    }
    fn solve_joint(&mut self, index: usize, dt: f32) {
        let joint = &self.joints[index];
        let Some(a) = self.bodies.iter().position(|b| b.id == joint.a) else {
            return;
        };
        let Some(b) = self.bodies.iter().position(|b| b.id == joint.b) else {
            return;
        };
        let pa = self.bodies[a].position + joint.local_a.rotate(self.bodies[a].angle);
        let pb = self.bodies[b].position + joint.local_b.rotate(self.bodies[b].angle);
        let d = pb - pa;
        let n = d.normalized();
        let ra = pa - self.bodies[a].position;
        let rb = pb - self.bodies[b].position;
        let k = self.bodies[a].inv_mass
            + self.bodies[b].inv_mass
            + self.bodies[a].inv_inertia * ra.cross(n).powi(2)
            + self.bodies[b].inv_inertia * rb.cross(n).powi(2);
        if k <= 0.0 {
            return;
        }
        let vn = (self.bodies[b].point_velocity(pb) - self.bodies[a].point_velocity(pa)).dot(n);
        let impulse = -(vn + 0.15 * (d.length() - joint.length) / dt) / k;
        self.bodies[a].impulse(pa, -n * impulse);
        self.bodies[b].impulse(pb, n * impulse);
        self.joints[index].impulse += impulse;
    }
    pub fn energy(&self) -> f32 {
        self.bodies
            .iter()
            .map(|b| {
                0.5 * b.mass * b.velocity.dot(b.velocity)
                    + if b.inv_inertia > 0.0 {
                        0.5 * b.angular_velocity.powi(2) / b.inv_inertia
                    } else {
                        0.0
                    }
            })
            .sum()
    }
    pub fn load_scene(&mut self, scene: u32) {
        *self = Self::new();
        self.add(
            Shape::Box {
                half: Vec2::new(9.0, 0.3),
            },
            Vec2::new(0.0, -0.3),
            0.0,
        );
        match scene {
            1 => {
                for i in 0..5 {
                    let x = -4.0 + i as f32 * 2.0;
                    let anchor = self.add(Shape::Circle { radius: 0.11 }, Vec2::new(x, 6.7), 0.0);
                    let bob =
                        self.add(Shape::Circle { radius: 0.42 }, Vec2::new(x + 0.6, 3.2), 2.0);
                    self.add_joint(anchor, bob, Vec2::new(x, 6.7), Vec2::new(x + 0.6, 3.2));
                }
            }
            2 => {
                for i in 0..3 {
                    let y = 1.3 + i as f32 * 1.8;
                    let angle = -0.14;
                    let id = self.add(
                        Shape::Box {
                            half: Vec2::new(5.8, 0.12),
                        },
                        Vec2::new(0.0, y),
                        0.0,
                    );
                    self.body_mut(id).unwrap().angle = angle;
                    self.body_mut(id).unwrap().friction = [0.02, 0.2, 0.9][i];
                    let pos = Vec2::new(-4.5, 0.55).rotate(angle) + Vec2::new(0.0, y);
                    let block = self.add(
                        Shape::Box {
                            half: Vec2::new(0.45, 0.4),
                        },
                        pos,
                        1.0,
                    );
                    let b = self.body_mut(block).unwrap();
                    b.angle = angle;
                    b.friction = [0.02, 0.2, 0.9][i];
                }
            }
            3 => {
                for i in 0..7 {
                    let id = self.add(
                        Shape::Circle {
                            radius: 0.3 + (i % 3) as f32 * 0.08,
                        },
                        Vec2::new(-4.5 + i as f32 * 1.4, 2.5 + (i % 2) as f32 * 2.0),
                        1.0,
                    );
                    self.body_mut(id).unwrap().restitution = 0.72;
                }
                let id = self.add(
                    Shape::Box {
                        half: Vec2::new(1.6, 0.15),
                    },
                    Vec2::new(0.0, 2.0),
                    0.0,
                );
                self.body_mut(id).unwrap().angle = 0.25;
            }
            _ => {
                for row in 0..7 {
                    for col in 0..3 {
                        let id = self.add(
                            Shape::Box {
                                half: Vec2::new(0.48, 0.38),
                            },
                            Vec2::new((col as f32 - 1.0) * 1.0, row as f32 * 0.78 + 0.4),
                            1.0,
                        );
                        self.body_mut(id).unwrap().restitution = 0.0;
                    }
                }
                let id = self.add(Shape::Circle { radius: 0.6 }, Vec2::new(-5.0, 3.0), 3.0);
                self.body_mut(id).unwrap().restitution = 0.45;
            }
        }
        // Side walls keep normal-speed interactions within the laboratory.
        self.add(
            Shape::Box {
                half: Vec2::new(0.25, 4.5),
            },
            Vec2::new(-9.25, 4.2),
            0.0,
        );
        self.add(
            Shape::Box {
                half: Vec2::new(0.25, 4.5),
            },
            Vec2::new(9.25, 4.2),
            0.0,
        );
    }
}
fn solve_contact(bodies: &mut [Body], c: &mut Contact) {
    let a = &bodies[c.a];
    let b = &bodies[c.b];
    let ra = c.point - a.position;
    let rb = c.point - b.position;
    let k = a.inv_mass
        + b.inv_mass
        + a.inv_inertia * ra.cross(c.normal).powi(2)
        + b.inv_inertia * rb.cross(c.normal).powi(2);
    if k <= 0.0 {
        return;
    }
    let vn = (b.point_velocity(c.point) - a.point_velocity(c.point)).dot(c.normal);
    let old = c.normal_impulse;
    c.normal_impulse = (old + (c.target_velocity - vn) / k).max(0.0);
    let p = c.normal * (c.normal_impulse - old);
    bodies[c.a].impulse(c.point, -p);
    bodies[c.b].impulse(c.point, p);
    let a = &bodies[c.a];
    let b = &bodies[c.b];
    let tangent = c.normal.perp();
    let kt = a.inv_mass
        + b.inv_mass
        + a.inv_inertia * ra.cross(tangent).powi(2)
        + b.inv_inertia * rb.cross(tangent).powi(2);
    if kt <= 0.0 {
        return;
    }
    let vt = (b.point_velocity(c.point) - a.point_velocity(c.point)).dot(tangent);
    let limit = (a.friction * b.friction).sqrt() * c.normal_impulse;
    let old = c.tangent_impulse;
    c.tangent_impulse = (old - vt / kt).clamp(-limit, limit);
    let p = tangent * (c.tangent_impulse - old);
    bodies[c.a].impulse(c.point, -p);
    bodies[c.b].impulse(c.point, p);
}

#[cfg(test)]
mod tests {
    use super::*;
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
        for _ in 0..600 {
            w.step(1.0 / 60.0);
        }
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
        w.add(
            Shape::Box {
                half: Vec2::new(20.0, 0.5),
            },
            Vec2::new(0.0, -0.5),
            0.0,
        );
        w.add(
            Shape::Box {
                half: Vec2::new(0.5, 0.5),
            },
            Vec2::new(0.0, 0.5),
            1.0,
        );
        w.bodies[1].velocity.x = 4.0;
        for _ in 0..120 {
            w.step(1.0 / 60.0);
        }
        assert!(w.bodies[1].velocity.x.abs() < 0.2);
    }
    #[test]
    fn joint_holds_its_length() {
        let mut w = World::new();
        let a = w.add(Shape::Circle { radius: 0.1 }, Vec2::new(0.0, 4.0), 0.0);
        let b = w.add(Shape::Circle { radius: 0.3 }, Vec2::new(1.0, 1.0), 1.0);
        w.add_joint(a, b, Vec2::new(0.0, 4.0), Vec2::new(1.0, 1.0));
        for _ in 0..600 {
            w.step(1.0 / 60.0);
        }
        let (pa, pb) = w.joint_anchors(&w.joints[0]).unwrap();
        assert!(((pb - pa).length() - 10.0_f32.sqrt()).abs() < 0.03);
    }
    #[test]
    fn scenes_survive_extended_simulation() {
        for scene in 0..4 {
            let mut w = World::new();
            w.load_scene(scene);
            for _ in 0..900 {
                w.step(1.0 / 60.0);
            }
            assert!(w.bodies.iter().all(|b| b.position.finite()
                && b.velocity.finite()
                && b.angular_velocity.is_finite()));
            assert!(w.energy() < 100.0, "scene {scene} energy {}", w.energy());
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
        w.step(f32::NAN);
        assert_eq!(w.tick, 0);
        assert_eq!(w.time, 0.0);
    }
}
