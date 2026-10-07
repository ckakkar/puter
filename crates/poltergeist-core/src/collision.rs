use crate::{Body, Shape, Vec2};

#[derive(Clone, Copy, Debug)]
pub struct GeometryContact {
    pub point: Vec2,
    pub normal: Vec2,
    pub penetration: f32,
    pub feature: u32,
}

pub fn bounds(b: &Body) -> (Vec2, Vec2) {
    let e = match b.shape {
        Shape::Circle { radius } => Vec2::new(radius, radius),
        Shape::Box { half } => {
            let (s, c) = b.angle.sin_cos();
            Vec2::new(
                c.abs() * half.x + s.abs() * half.y,
                s.abs() * half.x + c.abs() * half.y,
            )
        }
    };
    (b.position - e, b.position + e)
}

pub fn collide(a: &Body, b: &Body) -> Vec<GeometryContact> {
    match (a.shape, b.shape) {
        (Shape::Circle { radius: ra }, Shape::Circle { radius: rb }) => {
            let d = b.position - a.position;
            let l = d.length();
            if l > ra + rb {
                return vec![];
            }
            let n = d.normalized();
            vec![GeometryContact {
                point: a.position + n * (ra - (ra + rb - l) * 0.5),
                normal: n,
                penetration: ra + rb - l,
                feature: 0,
            }]
        }
        (Shape::Box { half }, Shape::Circle { radius }) => box_circle(a, b, half, radius),
        (Shape::Circle { radius }, Shape::Box { half }) => {
            let mut contacts = box_circle(b, a, half, radius);
            for c in &mut contacts {
                c.normal = -c.normal;
            }
            contacts
        }
        (Shape::Box { half: ha }, Shape::Box { half: hb }) => box_box(a, b, ha, hb),
    }
}

fn box_circle(a: &Body, b: &Body, half: Vec2, radius: f32) -> Vec<GeometryContact> {
    let local = (b.position - a.position).rotate(-a.angle);
    let q = Vec2::new(
        local.x.clamp(-half.x, half.x),
        local.y.clamp(-half.y, half.y),
    );
    let d = local - q;
    let l = d.length();
    let (n, p, penetration, feature) = if l > 1e-7 {
        if l > radius {
            return vec![];
        }
        (d / l, q, radius - l, 0)
    } else {
        let dx = half.x - local.x.abs();
        let dy = half.y - local.y.abs();
        if dx < dy {
            let sign = if local.x >= 0.0 { 1.0 } else { -1.0 };
            (
                Vec2::new(sign, 0.0),
                Vec2::new(sign * half.x, local.y),
                radius + dx,
                1,
            )
        } else {
            let sign = if local.y >= 0.0 { 1.0 } else { -1.0 };
            (
                Vec2::new(0.0, sign),
                Vec2::new(local.x, sign * half.y),
                radius + dy,
                2,
            )
        }
    };
    vec![GeometryContact {
        point: a.position + p.rotate(a.angle),
        normal: n.rotate(a.angle),
        penetration,
        feature,
    }]
}

#[derive(Clone, Copy)]
struct Vertex {
    p: Vec2,
    feature: u32,
}
fn clip(vertices: Vec<Vertex>, normal: Vec2, offset: f32, feature: u32) -> Vec<Vertex> {
    if vertices.len() < 2 {
        return vertices;
    }
    let a = vertices[0];
    let b = vertices[1];
    let da = a.p.dot(normal) - offset;
    let db = b.p.dot(normal) - offset;
    let mut out = Vec::with_capacity(2);
    if da <= 0.0 {
        out.push(a);
    }
    if db <= 0.0 {
        out.push(b);
    }
    if da * db < 0.0 {
        out.push(Vertex {
            p: a.p + (b.p - a.p) * (da / (da - db)),
            feature,
        });
    }
    out
}
fn box_box(a: &Body, b: &Body, ha: Vec2, hb: Vec2) -> Vec<GeometryContact> {
    let ax = Vec2::new(1.0, 0.0).rotate(a.angle);
    let ay = ax.perp();
    let bx = Vec2::new(1.0, 0.0).rotate(b.angle);
    let by = bx.perp();
    let axes = [ax, ay, bx, by];
    let d = b.position - a.position;
    let mut best = f32::INFINITY;
    let mut index = 0;
    let mut normal = Vec2::ZERO;
    for (i, &axis) in axes.iter().enumerate() {
        let ra = ha.x * axis.dot(ax).abs() + ha.y * axis.dot(ay).abs();
        let rb = hb.x * axis.dot(bx).abs() + hb.y * axis.dot(by).abs();
        let overlap = ra + rb - d.dot(axis).abs();
        if overlap < 0.0 {
            return vec![];
        }
        // A small hysteresis favors A's face near equal depths, stabilizing IDs.
        if overlap < best - 0.001 {
            best = overlap;
            index = i;
            normal = if d.dot(axis) >= 0.0 { axis } else { -axis };
        }
    }
    let (reference, incident, rhalf, ihalf, rnormal) = if index < 2 {
        (a, b, ha, hb, normal)
    } else {
        (b, a, hb, ha, -normal)
    };
    let rx = Vec2::new(1.0, 0.0).rotate(reference.angle);
    let ref_x = rnormal.dot(rx).abs() > 0.5;
    let face_extent = if ref_x { rhalf.x } else { rhalf.y };
    let side_extent = if ref_x { rhalf.y } else { rhalf.x };
    let face = reference.position + rnormal * face_extent;
    let tangent = rnormal.perp();
    let ix = Vec2::new(1.0, 0.0).rotate(incident.angle);
    let iy = ix.perp();
    let candidates = [ix, iy, -ix, -iy];
    let mut inc_index = 0;
    for i in 1..4 {
        if candidates[i].dot(rnormal) < candidates[inc_index].dot(rnormal) {
            inc_index = i;
        }
    }
    let inc_normal = candidates[inc_index];
    let (depth, width) = if inc_index % 2 == 0 {
        (ihalf.x, ihalf.y)
    } else {
        (ihalf.y, ihalf.x)
    };
    let center = incident.position + inc_normal * depth;
    let it = inc_normal.perp();
    let vertices = vec![
        Vertex {
            p: center - it * width,
            feature: 0,
        },
        Vertex {
            p: center + it * width,
            feature: 1,
        },
    ];
    let vertices = clip(vertices, tangent, face.dot(tangent) + side_extent, 2);
    let vertices = clip(vertices, -tangent, -face.dot(tangent) + side_extent, 3);
    let base = ((index as u32) * 8 + inc_index as u32) * 8
        + if rnormal.dot(axes[index]) < 0.0 {
            128
        } else {
            0
        };
    vertices
        .into_iter()
        .filter_map(|v| {
            let separation = (v.p - face).dot(rnormal);
            (separation <= 0.002).then_some(GeometryContact {
                point: v.p - rnormal * (separation * 0.5),
                normal,
                penetration: (-separation).max(0.0),
                feature: base + v.feature,
            })
        })
        .collect()
}
