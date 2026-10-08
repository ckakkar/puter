//! Small inspectable 2D rigid-body engine. Units: meters, kilograms, seconds, radians.
//! Single-threaded, discrete collisions; stable ordering, no cross-platform bit-exact promise.
pub mod collision;
pub mod history;
pub mod joint;
pub mod math;
mod scenes;
#[cfg(test)]
mod tests;

use history::{BodyState, Snapshot};
pub use joint::{Joint, JointKind, MAX_JOINTS};
use math::angular_velocity;
pub use math::Vec2;
pub use scenes::SCENE_COUNT;
use std::collections::BTreeMap;
use std::f32::consts::PI;

pub const MAX_BODIES: usize = 256;
pub const MIN_SIDES: u32 = 3;
pub const MAX_SIDES: u32 = 8;
/// Dynamic bodies that travel this far from the origin have left the laboratory.
pub const WORLD_LIMIT: f32 = 300.0;
/// Dynamic bodies that fall below this height are removed.
pub const FLOOR_LIMIT: f32 = -60.0;
/// Speed limits keep a single bad interaction from producing unusable states.
const MAX_SPEED: f32 = 150.0;
const MAX_ANGULAR_SPEED: f32 = 200.0;
const SLEEP_LINEAR: f32 = 0.05;
const SLEEP_ANGULAR: f32 = 0.05;
/// Seconds an island must stay below the sleep tolerances before it sleeps.
pub const TIME_TO_SLEEP: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Circle {
        radius: f32,
    },
    Box {
        half: Vec2,
    },
    /// Regular convex polygon described by its circumradius; see `collision::polygon_vertex`.
    Polygon {
        radius: f32,
        sides: u32,
    },
}
impl Shape {
    pub fn is_valid(&self) -> bool {
        match *self {
            Shape::Circle { radius } => radius.is_finite() && radius > 0.0,
            Shape::Box { half } => half.finite() && half.x > 0.0 && half.y > 0.0,
            Shape::Polygon { radius, sides } => {
                radius.is_finite() && radius > 0.0 && (MIN_SIDES..=MAX_SIDES).contains(&sides)
            }
        }
    }
    pub fn area(&self) -> f32 {
        match *self {
            Shape::Circle { radius } => PI * radius * radius,
            Shape::Box { half } => 4.0 * half.x * half.y,
            Shape::Polygon { radius, sides } => {
                0.5 * sides as f32 * radius * radius * (2.0 * PI / sides as f32).sin()
            }
        }
    }
    /// Rotational inertia about the centroid per kilogram.
    fn unit_inertia(&self) -> f32 {
        match *self {
            Shape::Circle { radius } => 0.5 * radius * radius,
            Shape::Box { half } => (half.x * half.x + half.y * half.y) / 3.0,
            Shape::Polygon { radius, sides } => {
                let c = (PI / sides as f32).cos();
                radius * radius * (1.0 + 2.0 * c * c) / 6.0
            }
        }
    }
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
    /// Sleeping dynamic bodies are neither integrated nor solved until something wakes them.
    pub awake: bool,
    pub sleep_time: f32,
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
            awake: true,
            sleep_time: 0.0,
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
        let inertia = self.mass * self.shape.unit_inertia();
        self.inv_inertia = if inertia > 0.0 { 1.0 / inertia } else { 0.0 };
        if self.inv_mass == 0.0 {
            self.velocity = Vec2::ZERO;
            self.angular_velocity = 0.0;
            self.awake = true;
        }
    }
    pub fn is_dynamic(&self) -> bool {
        self.inv_mass > 0.0
    }
    /// Dynamic and awake: moved by the integrator and the solver.
    pub fn is_active(&self) -> bool {
        self.inv_mass > 0.0 && self.awake
    }
    pub fn wake(&mut self) {
        self.awake = true;
        self.sleep_time = 0.0;
    }
    pub fn point_velocity(&self, p: Vec2) -> Vec2 {
        self.velocity + angular_velocity(self.angular_velocity, p - self.position)
    }
    pub fn impulse(&mut self, p: Vec2, j: Vec2) {
        self.velocity += j * self.inv_mass;
        self.angular_velocity += (p - self.position).cross(j) * self.inv_inertia;
    }
    /// Constraint impulses never move a sleeping body.
    fn solver_impulse(&mut self, p: Vec2, j: Vec2) {
        if self.awake {
            self.impulse(p, j);
        }
    }
    /// Inverse mass and inertia as seen by the solver: sleeping bodies act as immovable.
    fn solver_inverse(&self) -> (f32, f32) {
        if self.awake {
            (self.inv_mass, self.inv_inertia)
        } else {
            (0.0, 0.0)
        }
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
    /// Always sorted by ascending ID.
    pub bodies: Vec<Body>,
    pub contacts: Vec<Contact>,
    pub joints: Vec<Joint>,
    pub gravity: Vec2,
    pub iterations: usize,
    pub allow_sleep: bool,
    pub time: f32,
    pub tick: u64,
    pub candidate_pairs: usize,
    /// Total dynamic bodies removed for leaving the world.
    pub culled: u32,
    pub mouse: Option<MouseJoint>,
    next_id: u32,
    next_joint_id: u32,
    version: u64,
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
            allow_sleep: true,
            time: 0.0,
            tick: 0,
            candidate_pairs: 0,
            culled: 0,
            mouse: None,
            next_id: 1,
            next_joint_id: 1,
            version: 0,
            cache: BTreeMap::new(),
        }
    }
    /// Empties the world. The structure version keeps counting, so a timeline recorded
    /// in the old world can never be applied to the new one.
    pub fn clear(&mut self) {
        let version = self.version + 1;
        *self = Self::new();
        self.version = version;
    }
    /// Increments whenever bodies or joints are added or removed.
    pub fn structure_version(&self) -> u64 {
        self.version
    }
    pub fn index_of(&self, id: u32) -> Option<usize> {
        self.bodies.binary_search_by_key(&id, |b| b.id).ok()
    }
    pub fn body(&self, id: u32) -> Option<&Body> {
        self.index_of(id).map(|i| &self.bodies[i])
    }
    pub fn body_mut(&mut self, id: u32) -> Option<&mut Body> {
        self.index_of(id).map(|i| &mut self.bodies[i])
    }
    pub fn sleeping_count(&self) -> usize {
        self.bodies
            .iter()
            .filter(|b| b.is_dynamic() && !b.awake)
            .count()
    }

    pub fn add(&mut self, shape: Shape, p: Vec2, mass: f32) -> u32 {
        self.add_with_id(0, shape, p, mass)
    }
    /// Adds a body with a chosen ID (0 picks the next free one). Returns 0 on failure.
    pub fn add_with_id(&mut self, id: u32, shape: Shape, p: Vec2, mass: f32) -> u32 {
        if self.bodies.len() >= MAX_BODIES
            || !p.finite()
            || !mass.is_finite()
            || mass < 0.0
            || !shape.is_valid()
        {
            return 0;
        }
        let id = if id == 0 { self.next_id } else { id };
        if id == u32::MAX || self.index_of(id).is_some() {
            return 0;
        }
        let at = self.bodies.partition_point(|b| b.id < id);
        if at < self.bodies.len() {
            // Inserting shifts indices that displayed contacts refer to.
            self.contacts.clear();
        }
        self.bodies.insert(at, Body::new(id, shape, p, mass));
        self.next_id = self.next_id.max(id + 1);
        self.version += 1;
        id
    }
    pub fn remove(&mut self, id: u32) {
        let Some(index) = self.index_of(id) else {
            return;
        };
        self.wake_neighbors(index);
        self.joints.retain(|j| j.a != id && j.b != id);
        self.bodies.remove(index);
        self.contacts.retain(|c| c.a != index && c.b != index);
        for c in &mut self.contacts {
            c.a -= (c.a > index) as usize;
            c.b -= (c.b > index) as usize;
        }
        if self.mouse.is_some_and(|m| m.id == id) {
            self.mouse = None;
        }
        self.version += 1;
    }
    /// Topmost body at `p`, or 0.
    pub fn pick(&self, p: Vec2) -> u32 {
        self.pick_excluding(p, 0)
    }
    /// Topmost body at `p` other than `exclude`, or 0.
    pub fn pick_excluding(&self, p: Vec2, exclude: u32) -> u32 {
        self.bodies
            .iter()
            .rev()
            .find(|b| b.id != exclude && collision::contains(b, p))
            .map_or(0, |b| b.id)
    }
    fn wake(&mut self, index: usize) {
        if self.bodies[index].is_dynamic() {
            self.bodies[index].wake();
        }
    }
    /// Wakes `index` and everything touching or jointed to it.
    fn wake_neighbors(&mut self, index: usize) {
        self.wake(index);
        let id = self.bodies[index].id;
        let mut others: Vec<usize> = self
            .contacts
            .iter()
            .filter_map(|c| match (c.a == index, c.b == index) {
                (true, _) => Some(c.b),
                (_, true) => Some(c.a),
                _ => None,
            })
            .collect();
        for j in &self.joints {
            let other = if j.a == id {
                j.b
            } else if j.b == id {
                j.a
            } else {
                0
            };
            if let Some(i) = self.index_of(other) {
                others.push(i);
            }
        }
        for i in others {
            if i < self.bodies.len() {
                self.wake(i);
            }
        }
    }
    pub fn wake_all(&mut self) {
        for b in self.bodies.iter_mut().filter(|b| b.is_dynamic()) {
            b.wake();
        }
    }
    pub fn apply_impulse(&mut self, id: u32, p: Vec2, j: Vec2) {
        if !p.finite() || !j.finite() {
            return;
        }
        if let Some(i) = self.index_of(id) {
            self.wake(i);
            self.bodies[i].impulse(p, j);
        }
    }
    pub fn drag(&mut self, id: u32, p: Vec2) {
        if !p.finite() {
            return;
        }
        if let Some(i) = self.index_of(id).filter(|&i| self.bodies[i].is_dynamic()) {
            self.wake(i);
            let b = &self.bodies[i];
            self.mouse = Some(MouseJoint {
                id,
                local_anchor: (p - b.position).rotate(-b.angle),
                target: p,
            });
        }
    }
    /// Teleports a body and stops it, waking anything it touched.
    pub fn place(&mut self, id: u32, p: Vec2) {
        if !p.finite() {
            return;
        }
        if let Some(i) = self.index_of(id) {
            self.wake_neighbors(i);
            let b = &mut self.bodies[i];
            b.position = p;
            b.velocity = Vec2::ZERO;
            b.angular_velocity = 0.0;
        }
    }
    /// Sets orientation and motion; ignored for non-finite input.
    pub fn set_motion(&mut self, id: u32, angle: f32, velocity: Vec2, angular_velocity: f32) {
        if !angle.is_finite() || !velocity.finite() || !angular_velocity.is_finite() {
            return;
        }
        if let Some(i) = self.index_of(id) {
            self.wake_neighbors(i);
            let b = &mut self.bodies[i];
            b.angle = angle;
            if b.is_dynamic() {
                b.velocity = velocity;
                b.angular_velocity = angular_velocity;
            }
        }
    }
    /// Mass 0 makes a body static. Values are clamped to the sandbox's ranges.
    pub fn set_properties(&mut self, id: u32, mass: f32, friction: f32, restitution: f32) {
        if ![mass, friction, restitution].iter().all(|v| v.is_finite()) {
            return;
        }
        if let Some(i) = self.index_of(id) {
            self.wake_neighbors(i);
            let b = &mut self.bodies[i];
            b.set_mass(mass.clamp(0.0, 100.0));
            b.friction = friction.clamp(0.0, 1.5);
            b.restitution = restitution.clamp(0.0, 1.0);
            if b.is_dynamic() {
                b.wake();
            }
            if !b.is_dynamic() && self.mouse.is_some_and(|m| m.id == id) {
                self.mouse = None;
            }
        }
    }
    pub fn configure(&mut self, gravity: Vec2, iterations: usize, allow_sleep: bool) {
        if !gravity.finite() {
            return;
        }
        self.gravity = gravity;
        self.iterations = iterations.clamp(1, 32);
        self.allow_sleep = allow_sleep;
        self.wake_all();
    }

    /// Connects body `a` to body `b` (0 for the world) through world-space anchors.
    /// Distance joints take their length from the anchors. Returns the joint ID or 0.
    pub fn add_joint(&mut self, kind: JointKind, a: u32, b: u32, pa: Vec2, pb: Vec2) -> u32 {
        if !pa.finite() || !pb.finite() {
            return 0;
        }
        let (a, b, pa, pb) = if a == 0 {
            (b, a, pb, pa)
        } else {
            (a, b, pa, pb)
        };
        let local = |id: u32, p: Vec2| match self.body(id) {
            Some(body) => Some((p - body.position).rotate(-body.angle)),
            None if id == 0 => Some(p),
            None => None,
        };
        let (Some(la), Some(lb)) = (local(a, pa), local(b, pb)) else {
            return 0;
        };
        let length = if kind.is_distance() {
            (pb - pa).length()
        } else {
            0.0
        };
        self.add_joint_local(0, kind, a, b, la, lb, length)
    }
    /// Adds a joint from body-local anchors (a world point when `b` is 0), as stored in scenes.
    #[allow(clippy::too_many_arguments)]
    pub fn add_joint_local(
        &mut self,
        id: u32,
        kind: JointKind,
        a: u32,
        b: u32,
        local_a: Vec2,
        local_b: Vec2,
        length: f32,
    ) -> u32 {
        let id = if id == 0 { self.next_joint_id } else { id };
        let valid_length = length.is_finite()
            && if kind.is_distance() {
                length >= 0.01
            } else {
                length >= 0.0
            };
        let dynamic = |id: u32| self.body(id).is_some_and(|b| b.is_dynamic());
        if self.joints.len() >= MAX_JOINTS
            || id == u32::MAX
            || self.joints.iter().any(|j| j.id == id)
            || a == b
            || self.body(a).is_none()
            || (b != 0 && self.body(b).is_none())
            || !(dynamic(a) || dynamic(b))
            || !local_a.finite()
            || !local_b.finite()
            || !valid_length
        {
            return 0;
        }
        self.joints.push(Joint {
            id,
            kind,
            a,
            b,
            local_a,
            local_b,
            length,
            impulse: 0.0,
            motor_impulse: 0.0,
            point_impulse: Vec2::ZERO,
        });
        self.next_joint_id = self.next_joint_id.max(id + 1);
        self.wake_joint(id);
        self.version += 1;
        id
    }
    fn wake_joint(&mut self, id: u32) {
        if let Some(j) = self.joints.iter().find(|j| j.id == id) {
            let (a, b) = (j.a, j.b);
            for i in [self.index_of(a), self.index_of(b)].into_iter().flatten() {
                self.wake(i);
            }
        }
    }
    pub fn remove_joint(&mut self, id: u32) {
        if self.joints.iter().any(|j| j.id == id) {
            self.wake_joint(id);
            self.joints.retain(|j| j.id != id);
            self.version += 1;
        }
    }
    /// Updates a joint's length and kind parameters; the kind itself cannot change.
    pub fn set_joint(&mut self, id: u32, length: f32, p1: f32, p2: f32) {
        let Some(index) = self.joints.iter().position(|j| j.id == id) else {
            return;
        };
        let joint = &mut self.joints[index];
        if let Some(kind) = JointKind::from_code(joint.kind.code(), p1, p2) {
            joint.kind = kind;
        }
        if joint.kind.is_distance() && length.is_finite() {
            joint.length = length.clamp(0.01, 2.0 * WORLD_LIMIT);
        }
        self.wake_joint(id);
    }
    pub fn joint_anchors(&self, j: &Joint) -> Option<(Vec2, Vec2)> {
        let a = self.body(j.a)?;
        let pb = if j.b == 0 {
            j.local_b
        } else {
            let b = self.body(j.b)?;
            b.position + j.local_b.rotate(b.angle)
        };
        Some((a.position + j.local_a.rotate(a.angle), pb))
    }
    /// Body indices for each joint; `None` if a body is missing.
    fn joint_links(&self) -> Vec<Option<(usize, Option<usize>)>> {
        self.joints
            .iter()
            .map(|j| {
                let a = self.index_of(j.a)?;
                if j.b == 0 {
                    Some((a, None))
                } else {
                    Some((a, Some(self.index_of(j.b)?)))
                }
            })
            .collect()
    }

    /// Candidate pairs from a sweep along x, returned in ascending (a, b) order so
    /// the solver visits contacts in the same order as an exhaustive search would.
    fn broad_phase(&mut self) -> Vec<(usize, usize)> {
        let bounds: Vec<_> = self.bodies.iter().map(collision::bounds).collect();
        let mut order: Vec<usize> = (0..self.bodies.len()).collect();
        order.sort_unstable_by(|&i, &j| bounds[i].0.x.total_cmp(&bounds[j].0.x).then(i.cmp(&j)));
        let mut open: Vec<usize> = vec![];
        let mut pairs = vec![];
        for &i in &order {
            let (imin, imax) = bounds[i];
            open.retain(|&j| bounds[j].1.x >= imin.x);
            for &j in &open {
                let (jmin, jmax) = bounds[j];
                if jmax.y < imin.y || imax.y < jmin.y {
                    continue;
                }
                if !self.bodies[i].is_dynamic() && !self.bodies[j].is_dynamic() {
                    continue;
                }
                pairs.push((i.min(j), i.max(j)));
            }
            open.push(i);
        }
        pairs.sort_unstable();
        self.candidate_pairs = pairs.len();
        pairs
    }
    fn detect(&mut self) -> Vec<Contact> {
        let mut contacts = vec![];
        for (a, b) in self.broad_phase() {
            let ba = &self.bodies[a];
            let bb = &self.bodies[b];
            for g in collision::collide(ba, bb) {
                let cached = self
                    .cache
                    .get(&(ba.id, bb.id, g.feature))
                    .filter(|c| c.normal.dot(g.normal) > 0.95);
                let vn = (bb.point_velocity(g.point) - ba.point_velocity(g.point)).dot(g.normal);
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
        contacts
    }
    /// Recomputes displayed contacts without advancing time (impulses start at zero).
    pub fn refresh_contacts(&mut self) {
        self.cache.clear();
        self.contacts = self.detect();
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
        self.cull();
    }
    fn substep(&mut self, dt: f32) {
        for b in self.bodies.iter_mut().filter(|b| b.is_active()) {
            b.velocity += self.gravity * dt;
        }
        if let Some(mouse) = self.mouse {
            if let Some(i) = self.index_of(mouse.id) {
                let b = &mut self.bodies[i];
                b.wake();
                let p = b.position + mouse.local_anchor.rotate(b.angle);
                let spring = (mouse.target - p) * 55.0 - b.point_velocity(p) * 10.0;
                // Cap acceleration so distant pointer jumps cannot launch bodies arbitrarily fast.
                let force = spring * (80.0 / spring.length().max(80.0));
                b.impulse(p, force * (b.mass * dt));
            }
        }
        let mut contacts = self.detect();
        let links = self.joint_links();
        self.wake_touched(&contacts, &links);
        let solves =
            |bodies: &[Body], a: usize, b: usize| bodies[a].is_active() || bodies[b].is_active();
        for c in &contacts {
            if !solves(&self.bodies, c.a, c.b) {
                continue;
            }
            let impulse = c.normal * c.normal_impulse + c.normal.perp() * c.tangent_impulse;
            self.bodies[c.a].solver_impulse(c.point, -impulse);
            self.bodies[c.b].solver_impulse(c.point, impulse);
        }
        for joint in &mut self.joints {
            joint::reset(joint);
        }
        let joint_active: Vec<bool> = links
            .iter()
            .map(|l| {
                l.is_some_and(|(a, b)| {
                    self.bodies[a].is_active() || b.is_some_and(|b| self.bodies[b].is_active())
                })
            })
            .collect();
        for _ in 0..self.iterations.clamp(1, 32) {
            for c in &mut contacts {
                if solves(&self.bodies, c.a, c.b) {
                    solve_contact(&mut self.bodies, c);
                }
            }
            for (i, link) in links.iter().enumerate() {
                if let (Some((a, b)), true) = (*link, joint_active[i]) {
                    joint::solve(&mut self.bodies, &mut self.joints[i], a, b, dt);
                }
            }
        }
        for b in self.bodies.iter_mut().filter(|b| b.is_active()) {
            let speed = b.velocity.length();
            if speed > MAX_SPEED {
                b.velocity = b.velocity * (MAX_SPEED / speed);
            }
            b.angular_velocity = b
                .angular_velocity
                .clamp(-MAX_ANGULAR_SPEED, MAX_ANGULAR_SPEED);
            b.position += b.velocity * dt;
            b.angle += b.angular_velocity * dt;
        }
        // Separate positional projection limits energy introduced by overlap correction.
        // A manifold's points are consecutive, so each pair is corrected once per pass.
        for _ in 0..4 {
            for (i, c) in contacts.iter().enumerate() {
                if i > 0 && contacts[i - 1].a == c.a && contacts[i - 1].b == c.b {
                    continue;
                }
                if !solves(&self.bodies, c.a, c.b) {
                    continue;
                }
                for g in collision::collide(&self.bodies[c.a], &self.bodies[c.b]) {
                    let a = &self.bodies[c.a];
                    let b = &self.bodies[c.b];
                    let (am, ai) = a.solver_inverse();
                    let (bm, bi) = b.solver_inverse();
                    let ra = g.point - a.position;
                    let rb = g.point - b.position;
                    let k =
                        am + bm + ai * ra.cross(g.normal).powi(2) + bi * rb.cross(g.normal).powi(2);
                    if k <= 0.0 {
                        continue;
                    }
                    let magnitude = (0.25 * (g.penetration - 0.004).max(0.0)).min(0.08) / k;
                    let p = g.normal * magnitude;
                    self.bodies[c.a].position -= p * am;
                    self.bodies[c.a].angle -= ra.cross(p) * ai;
                    self.bodies[c.b].position += p * bm;
                    self.bodies[c.b].angle += rb.cross(p) * bi;
                }
            }
        }
        self.update_sleep(&contacts, &links, dt);
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
    /// Sleeping bodies touched or pulled by awake ones wake, transitively, before the
    /// solver runs, so a disturbed island responds as a whole.
    fn wake_touched(&mut self, contacts: &[Contact], links: &[Option<(usize, Option<usize>)>]) {
        loop {
            let mut changed = false;
            let pairs = contacts
                .iter()
                .map(|c| (c.a, Some(c.b)))
                .chain(links.iter().flatten().copied());
            for (a, b) in pairs {
                let Some(b) = b else { continue };
                for (x, y) in [(a, b), (b, a)] {
                    if self.bodies[x].is_active()
                        && self.bodies[y].is_dynamic()
                        && !self.bodies[y].awake
                    {
                        self.bodies[y].wake();
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }
    /// Islands of touching or jointed dynamic bodies sleep together once every member
    /// has been nearly still for `TIME_TO_SLEEP`, and wake together otherwise.
    fn update_sleep(
        &mut self,
        contacts: &[Contact],
        links: &[Option<(usize, Option<usize>)>],
        dt: f32,
    ) {
        if !self.allow_sleep {
            for b in &mut self.bodies {
                b.awake = true;
                b.sleep_time = 0.0;
            }
            return;
        }
        for b in self.bodies.iter_mut().filter(|b| b.is_active()) {
            let moving = b.velocity.length_squared() > SLEEP_LINEAR * SLEEP_LINEAR
                || b.angular_velocity.abs() > SLEEP_ANGULAR;
            b.sleep_time = if moving { 0.0 } else { b.sleep_time + dt };
        }
        let mut busy: Vec<usize> = vec![];
        if let Some(i) = self.mouse.and_then(|m| self.index_of(m.id)) {
            busy.push(i);
        }
        for (j, link) in self.joints.iter().zip(links) {
            if let (
                JointKind::Pin {
                    motor_speed,
                    max_torque,
                },
                Some((a, b)),
            ) = (j.kind, link)
            {
                if max_torque > 0.0 && motor_speed != 0.0 {
                    busy.push(*a);
                    busy.extend(*b);
                }
            }
        }
        for i in busy {
            if self.bodies[i].is_dynamic() {
                self.bodies[i].wake();
            }
        }
        let n = self.bodies.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn root(parent: &mut [usize], mut i: usize) -> usize {
            while parent[i] != i {
                parent[i] = parent[parent[i]];
                i = parent[i];
            }
            i
        }
        let edges = contacts
            .iter()
            .map(|c| (c.a, Some(c.b)))
            .chain(links.iter().flatten().copied());
        for (a, b) in edges {
            let Some(b) = b else { continue };
            if self.bodies[a].is_dynamic() && self.bodies[b].is_dynamic() {
                let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
                parent[ra] = rb;
            }
        }
        let mut island_time = vec![f32::INFINITY; n];
        for i in 0..n {
            if self.bodies[i].is_dynamic() {
                let r = root(&mut parent, i);
                island_time[r] = island_time[r].min(self.bodies[i].sleep_time);
            }
        }
        for i in 0..n {
            if !self.bodies[i].is_dynamic() {
                continue;
            }
            let r = root(&mut parent, i);
            let b = &mut self.bodies[i];
            if island_time[r] >= TIME_TO_SLEEP {
                b.awake = false;
                b.velocity = Vec2::ZERO;
                b.angular_velocity = 0.0;
            } else if !b.awake {
                b.wake();
            }
        }
    }
    /// Removes dynamic bodies that have left the laboratory.
    fn cull(&mut self) {
        let lost: Vec<u32> = self
            .bodies
            .iter()
            .filter(|b| {
                b.is_dynamic()
                    && (!b.position.finite()
                        || b.position.y < FLOOR_LIMIT
                        || b.position.y > WORLD_LIMIT
                        || b.position.x.abs() > WORLD_LIMIT)
            })
            .map(|b| b.id)
            .collect();
        for id in lost {
            self.remove(id);
            self.culled += 1;
        }
    }

    /// Kinetic energy of all bodies (J).
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
    /// Gravitational energy of dynamic bodies relative to the origin, plus spring energy (J).
    pub fn potential_energy(&self) -> f32 {
        let gravity: f32 = self
            .bodies
            .iter()
            .filter(|b| b.is_dynamic())
            .map(|b| -b.mass * self.gravity.dot(b.position))
            .sum();
        let springs: f32 = self
            .joints
            .iter()
            .zip(self.joint_links())
            .filter_map(|(j, l)| l.map(|(a, b)| joint::potential_energy(&self.bodies, j, a, b)))
            .sum();
        gravity + springs
    }

    pub(crate) fn capture(&self, mut into: Vec<BodyState>) -> Snapshot {
        into.clear();
        into.extend(self.bodies.iter().map(|b| BodyState {
            position: b.position,
            angle: b.angle,
            velocity: b.velocity,
            angular_velocity: b.angular_velocity,
            awake: b.awake,
            sleep_time: b.sleep_time,
        }));
        Snapshot {
            time: self.time,
            tick: self.tick,
            version: self.version,
            bodies: into,
        }
    }
    pub(crate) fn restore(&mut self, s: &Snapshot) {
        for (b, state) in self.bodies.iter_mut().zip(&s.bodies) {
            b.position = state.position;
            b.angle = state.angle;
            b.velocity = state.velocity;
            b.angular_velocity = state.angular_velocity;
            b.awake = state.awake;
            b.sleep_time = state.sleep_time;
        }
        self.time = s.time;
        self.tick = s.tick;
        self.mouse = None;
        for j in &mut self.joints {
            joint::reset(j);
        }
        self.refresh_contacts();
    }
}
fn solve_contact(bodies: &mut [Body], c: &mut Contact) {
    let a = &bodies[c.a];
    let b = &bodies[c.b];
    let (am, ai) = a.solver_inverse();
    let (bm, bi) = b.solver_inverse();
    let ra = c.point - a.position;
    let rb = c.point - b.position;
    let k = am + bm + ai * ra.cross(c.normal).powi(2) + bi * rb.cross(c.normal).powi(2);
    if k <= 0.0 {
        return;
    }
    let vn = (b.point_velocity(c.point) - a.point_velocity(c.point)).dot(c.normal);
    let old = c.normal_impulse;
    c.normal_impulse = (old + (c.target_velocity - vn) / k).max(0.0);
    let p = c.normal * (c.normal_impulse - old);
    bodies[c.a].solver_impulse(c.point, -p);
    bodies[c.b].solver_impulse(c.point, p);
    let a = &bodies[c.a];
    let b = &bodies[c.b];
    let tangent = c.normal.perp();
    let kt = am + bm + ai * ra.cross(tangent).powi(2) + bi * rb.cross(tangent).powi(2);
    if kt <= 0.0 {
        return;
    }
    let vt = (b.point_velocity(c.point) - a.point_velocity(c.point)).dot(tangent);
    let limit = (a.friction * b.friction).sqrt() * c.normal_impulse;
    let old = c.tangent_impulse;
    c.tangent_impulse = (old - vt / kt).clamp(-limit, limit);
    let p = tangent * (c.tangent_impulse - old);
    bodies[c.a].solver_impulse(c.point, -p);
    bodies[c.b].solver_impulse(c.point, p);
}
