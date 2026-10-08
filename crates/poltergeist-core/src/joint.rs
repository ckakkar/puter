//! Constraints between two bodies, or between a body and a fixed world point.
use crate::math::angular_velocity;
use crate::{Body, Vec2};
use std::f32::consts::TAU;

pub const MAX_JOINTS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JointKind {
    /// Holds the anchors at a fixed distance.
    Rod,
    /// Lets the anchors approach freely but never separate beyond the length.
    Rope,
    /// A soft distance constraint: frequency in hertz, damping ratio (1 = critical).
    Spring { frequency: f32, damping: f32 },
    /// Holds the anchors together, leaving rotation free. An optional motor drives
    /// body A relative to body B at `motor_speed` rad/s, limited to `max_torque` N·m.
    Pin { motor_speed: f32, max_torque: f32 },
}
impl JointKind {
    pub fn code(self) -> u32 {
        match self {
            JointKind::Rod => 0,
            JointKind::Rope => 1,
            JointKind::Spring { .. } => 2,
            JointKind::Pin { .. } => 3,
        }
    }
    /// The two tunable parameters, packed for the scalar ABI.
    pub fn params(self) -> (f32, f32) {
        match self {
            JointKind::Rod | JointKind::Rope => (0.0, 0.0),
            JointKind::Spring { frequency, damping } => (frequency, damping),
            JointKind::Pin {
                motor_speed,
                max_torque,
            } => (motor_speed, max_torque),
        }
    }
    pub fn from_code(code: u32, p1: f32, p2: f32) -> Option<Self> {
        if !p1.is_finite() || !p2.is_finite() {
            return None;
        }
        Some(match code {
            0 => JointKind::Rod,
            1 => JointKind::Rope,
            2 => JointKind::Spring {
                frequency: p1.clamp(0.05, 30.0),
                damping: p2.clamp(0.0, 5.0),
            },
            3 => JointKind::Pin {
                motor_speed: p1.clamp(-50.0, 50.0),
                max_torque: p2.clamp(0.0, 10_000.0),
            },
            _ => return None,
        })
    }
    pub fn is_distance(self) -> bool {
        !matches!(self, JointKind::Pin { .. })
    }
}

#[derive(Clone, Debug)]
pub struct Joint {
    pub id: u32,
    pub kind: JointKind,
    /// Body ID of the first anchor. Always a real body.
    pub a: u32,
    /// Body ID of the second anchor, or 0 for a fixed point in the world.
    pub b: u32,
    pub local_a: Vec2,
    /// Body-local anchor on B, or a world point when `b == 0`.
    pub local_b: Vec2,
    pub length: f32,
    /// Accumulated impulse from the latest substep (N·s): signed along A→B for
    /// distance joints, the magnitude of the point impulse for pins.
    pub impulse: f32,
    pub(crate) motor_impulse: f32,
    pub(crate) point_impulse: Vec2,
}

/// One side of a joint, resolved for a substep. A missing index means the world.
#[derive(Clone, Copy)]
struct Side {
    index: Option<usize>,
    r: Vec2,
    point: Vec2,
    inv_mass: f32,
    inv_inertia: f32,
}
impl Side {
    fn new(bodies: &[Body], index: Option<usize>, local: Vec2) -> Self {
        match index {
            Some(i) => {
                let b = &bodies[i];
                let r = local.rotate(b.angle);
                let awake = b.awake;
                Side {
                    index,
                    r,
                    point: b.position + r,
                    inv_mass: if awake { b.inv_mass } else { 0.0 },
                    inv_inertia: if awake { b.inv_inertia } else { 0.0 },
                }
            }
            None => Side {
                index,
                r: Vec2::ZERO,
                point: local,
                inv_mass: 0.0,
                inv_inertia: 0.0,
            },
        }
    }
    fn velocity(&self, bodies: &[Body]) -> Vec2 {
        self.index.map_or(Vec2::ZERO, |i| {
            bodies[i].velocity + angular_velocity(bodies[i].angular_velocity, self.r)
        })
    }
    fn omega(&self, bodies: &[Body]) -> f32 {
        self.index.map_or(0.0, |i| bodies[i].angular_velocity)
    }
    fn apply(&self, bodies: &mut [Body], j: Vec2) {
        if let Some(i) = self.index {
            bodies[i].velocity += j * self.inv_mass;
            bodies[i].angular_velocity += self.r.cross(j) * self.inv_inertia;
        }
    }
    fn apply_angular(&self, bodies: &mut [Body], l: f32) {
        if let Some(i) = self.index {
            bodies[i].angular_velocity += l * self.inv_inertia;
        }
    }
}

fn effective_mass(a: &Side, b: &Side, n: Vec2) -> f32 {
    a.inv_mass
        + b.inv_mass
        + a.inv_inertia * a.r.cross(n).powi(2)
        + b.inv_inertia * b.r.cross(n).powi(2)
}

/// Mass-normalized spring stiffness (N/m) and damping (N·s/m) for the current geometry.
fn spring_coefficients(k: f32, frequency: f32, damping: f32) -> (f32, f32) {
    let mass = 1.0 / k;
    let omega = TAU * frequency;
    (mass * omega * omega, 2.0 * mass * damping * omega)
}

pub(crate) fn reset(joint: &mut Joint) {
    joint.impulse = 0.0;
    joint.motor_impulse = 0.0;
    joint.point_impulse = Vec2::ZERO;
}

/// One sequential-impulse iteration for a joint.
pub(crate) fn solve(bodies: &mut [Body], joint: &mut Joint, ia: usize, ib: Option<usize>, dt: f32) {
    let a = Side::new(bodies, Some(ia), joint.local_a);
    let b = Side::new(bodies, ib, joint.local_b);
    match joint.kind {
        JointKind::Pin {
            motor_speed,
            max_torque,
        } => {
            let angular = a.inv_inertia + b.inv_inertia;
            if max_torque > 0.0 && angular > 0.0 {
                let cdot = a.omega(bodies) - b.omega(bodies) - motor_speed;
                let old = joint.motor_impulse;
                let limit = max_torque * dt;
                joint.motor_impulse = (old - cdot / angular).clamp(-limit, limit);
                let l = joint.motor_impulse - old;
                a.apply_angular(bodies, l);
                b.apply_angular(bodies, -l);
            }
            // 2×2 effective mass of the point constraint.
            let (m, ra, rb) = (a.inv_mass + b.inv_mass, a.r, b.r);
            let k11 = m + a.inv_inertia * ra.y * ra.y + b.inv_inertia * rb.y * rb.y;
            let k12 = -a.inv_inertia * ra.x * ra.y - b.inv_inertia * rb.x * rb.y;
            let k22 = m + a.inv_inertia * ra.x * ra.x + b.inv_inertia * rb.x * rb.x;
            let det = k11 * k22 - k12 * k12;
            if det.abs() < 1e-12 {
                return;
            }
            let c = b.point - a.point;
            let cdot = b.velocity(bodies) - a.velocity(bodies);
            let rhs = -(cdot + c * (0.2 / dt));
            let j = Vec2::new(k22 * rhs.x - k12 * rhs.y, k11 * rhs.y - k12 * rhs.x) / det;
            a.apply(bodies, -j);
            b.apply(bodies, j);
            joint.point_impulse += j;
            joint.impulse = joint.point_impulse.length();
        }
        kind => {
            let d = b.point - a.point;
            let n = d.normalized();
            let k = effective_mass(&a, &b, n);
            if k <= 0.0 {
                return;
            }
            let c = d.length() - joint.length;
            let vn = (b.velocity(bodies) - a.velocity(bodies)).dot(n);
            let lambda = match kind {
                JointKind::Rope => {
                    // Speculative when slack: allow approach up to the limit without bias.
                    let bias = if c > 0.0 { 0.15 * c / dt } else { c / dt };
                    let old = joint.impulse;
                    joint.impulse = (old - (vn + bias) / k).min(0.0);
                    joint.impulse - old
                }
                JointKind::Spring { frequency, damping } => {
                    let (stiffness, damper) = spring_coefficients(k, frequency, damping);
                    let gamma = dt * (damper + dt * stiffness);
                    let gamma = if gamma > 0.0 { 1.0 / gamma } else { 0.0 };
                    let bias = c * dt * stiffness * gamma;
                    let lambda = -(vn + bias + gamma * joint.impulse) / (k + gamma);
                    joint.impulse += lambda;
                    lambda
                }
                _ => {
                    let lambda = -(vn + 0.15 * c / dt) / k;
                    joint.impulse += lambda;
                    lambda
                }
            };
            a.apply(bodies, -n * lambda);
            b.apply(bodies, n * lambda);
        }
    }
}

/// Elastic energy stored in a spring joint (J); zero for rigid joints.
pub(crate) fn potential_energy(
    bodies: &[Body],
    joint: &Joint,
    ia: usize,
    ib: Option<usize>,
) -> f32 {
    let JointKind::Spring { frequency, damping } = joint.kind else {
        return 0.0;
    };
    // Measure against true masses, even for sleeping bodies.
    let side = |index: Option<usize>, local: Vec2| match index {
        Some(i) => {
            let body = &bodies[i];
            let r = local.rotate(body.angle);
            Side {
                index,
                r,
                point: body.position + r,
                inv_mass: body.inv_mass,
                inv_inertia: body.inv_inertia,
            }
        }
        None => Side::new(bodies, None, local),
    };
    let a = side(Some(ia), joint.local_a);
    let b = side(ib, joint.local_b);
    let d = b.point - a.point;
    let k = effective_mass(&a, &b, d.normalized());
    if k <= 0.0 {
        return 0.0;
    }
    let (stiffness, _) = spring_coefficients(k, frequency, damping);
    0.5 * stiffness * (d.length() - joint.length).powi(2)
}
