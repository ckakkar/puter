use crate::{Body, Shape, Vec2, MAX_SIDES};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug)]
pub struct GeometryContact {
    pub point: Vec2,
    pub normal: Vec2,
    pub penetration: f32,
    pub feature: u32,
}

/// Local vertex `i` of a regular polygon. Vertices run counter-clockwise and the
/// closing edge (last vertex to first) is horizontal, so an unrotated polygon
/// rests on a flat bottom.
pub fn polygon_vertex(radius: f32, sides: u32, i: u32) -> Vec2 {
    let n = sides as f32;
    let theta = -PI / 2.0 + PI / n + TAU * i as f32 / n;
    Vec2::new(radius * theta.cos(), radius * theta.sin())
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
        Shape::Polygon { radius, sides } => {
            let mut lo = Vec2::new(f32::INFINITY, f32::INFINITY);
            let mut hi = -lo;
            for i in 0..sides {
                let v = polygon_vertex(radius, sides, i).rotate(b.angle);
                lo = Vec2::new(lo.x.min(v.x), lo.y.min(v.y));
                hi = Vec2::new(hi.x.max(v.x), hi.y.max(v.y));
            }
            return (b.position + lo, b.position + hi);
        }
    };
    (b.position - e, b.position + e)
}

/// Whether world point `p` lies inside the body.
pub fn contains(b: &Body, p: Vec2) -> bool {
    let q = (p - b.position).rotate(-b.angle);
    match b.shape {
        Shape::Circle { radius } => q.dot(q) <= radius * radius,
        Shape::Box { half } => q.x.abs() <= half.x && q.y.abs() <= half.y,
        Shape::Polygon { radius, sides } => (0..sides).all(|i| {
            let v1 = polygon_vertex(radius, sides, i);
            let v2 = polygon_vertex(radius, sides, (i + 1) % sides);
            (v2 - v1).cross(q - v1) >= 0.0
        }),
    }
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
        (Shape::Circle { radius }, Shape::Box { half }) => flipped(box_circle(b, a, half, radius)),
        (Shape::Box { half: ha }, Shape::Box { half: hb }) => box_box(a, b, ha, hb),
        (Shape::Polygon { .. }, Shape::Circle { radius }) => polygon_circle(a, b, radius),
        (Shape::Circle { radius }, Shape::Polygon { .. }) => flipped(polygon_circle(b, a, radius)),
        _ => polygon_polygon(a, b),
    }
}

fn flipped(mut contacts: Vec<GeometryContact>) -> Vec<GeometryContact> {
    for c in &mut contacts {
        c.normal = -c.normal;
    }
    contacts
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

/// Cache key for a box-box manifold point. Every field gets its own bit range:
/// vertex (0..4) | incident face (0..4) << 2 | reference axis (0..4) << 4 | flip << 6.
pub(crate) fn box_feature(axis: usize, incident: usize, flip: bool, vertex: u32) -> u32 {
    ((flip as u32) << 6) | ((axis as u32) << 4) | ((incident as u32) << 2) | vertex
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
    let flip = rnormal.dot(axes[index]) < 0.0;
    vertices
        .into_iter()
        .filter_map(|v| {
            let separation = (v.p - face).dot(rnormal);
            (separation <= 0.002).then_some(GeometryContact {
                point: v.p - rnormal * (separation * 0.5),
                normal,
                penetration: (-separation).max(0.0),
                feature: box_feature(index, inc_index, flip, v.feature),
            })
        })
        .collect()
}

/// A convex outline in world space: counter-clockwise vertices with outward edge normals.
struct Poly {
    v: [Vec2; MAX_SIDES as usize],
    n: [Vec2; MAX_SIDES as usize],
    count: usize,
}
impl Poly {
    fn of(b: &Body) -> Self {
        let mut local = [Vec2::ZERO; MAX_SIDES as usize];
        let count = match b.shape {
            Shape::Box { half } => {
                local[..4].copy_from_slice(&[
                    Vec2::new(-half.x, -half.y),
                    Vec2::new(half.x, -half.y),
                    Vec2::new(half.x, half.y),
                    Vec2::new(-half.x, half.y),
                ]);
                4
            }
            Shape::Polygon { radius, sides } => {
                for i in 0..sides {
                    local[i as usize] = polygon_vertex(radius, sides, i);
                }
                sides as usize
            }
            Shape::Circle { .. } => unreachable!("circles are not polygons"),
        };
        let mut poly = Poly {
            v: [Vec2::ZERO; MAX_SIDES as usize],
            n: [Vec2::ZERO; MAX_SIDES as usize],
            count,
        };
        for i in 0..count {
            let e = local[(i + 1) % count] - local[i];
            poly.v[i] = b.position + local[i].rotate(b.angle);
            poly.n[i] = Vec2::new(e.y, -e.x).normalized().rotate(b.angle);
        }
        poly
    }
    fn next(&self, i: usize) -> usize {
        (i + 1) % self.count
    }
}

/// The edge of `p1` along which `p2` is least deep, and that separation.
fn max_separation(p1: &Poly, p2: &Poly) -> (usize, f32) {
    let mut best = (0, f32::NEG_INFINITY);
    for i in 0..p1.count {
        let s = (0..p2.count)
            .map(|j| p1.n[i].dot(p2.v[j] - p1.v[i]))
            .fold(f32::INFINITY, f32::min);
        if s > best.1 {
            best = (i, s);
        }
    }
    best
}

fn polygon_polygon(a: &Body, b: &Body) -> Vec<GeometryContact> {
    let pa = Poly::of(a);
    let pb = Poly::of(b);
    let (edge_a, sep_a) = max_separation(&pa, &pb);
    if sep_a > 0.0 {
        return vec![];
    }
    let (edge_b, sep_b) = max_separation(&pb, &pa);
    if sep_b > 0.0 {
        return vec![];
    }
    // Same hysteresis as box_box: prefer A's face unless B's is clearly shallower.
    let flip = sep_b > sep_a + 0.001;
    let (reference, incident, edge) = if flip {
        (&pb, &pa, edge_b)
    } else {
        (&pa, &pb, edge_a)
    };
    let rnormal = reference.n[edge];
    let inc_edge = (0..incident.count)
        .min_by(|&i, &j| {
            rnormal
                .dot(incident.n[i])
                .total_cmp(&rnormal.dot(incident.n[j]))
        })
        .unwrap_or(0);
    let v11 = reference.v[edge];
    let v12 = reference.v[reference.next(edge)];
    let tangent = (v12 - v11).normalized();
    let vertices = vec![
        Vertex {
            p: incident.v[inc_edge],
            feature: 0,
        },
        Vertex {
            p: incident.v[incident.next(inc_edge)],
            feature: 1,
        },
    ];
    let vertices = clip(vertices, -tangent, -tangent.dot(v11), 2);
    let vertices = clip(vertices, tangent, tangent.dot(v12), 3);
    let normal = if flip { -rnormal } else { rnormal };
    // flip | reference edge | incident edge | vertex, each in its own bit range.
    let base = ((flip as u32) << 10) | ((edge as u32) << 6) | ((inc_edge as u32) << 2);
    vertices
        .into_iter()
        .filter_map(|v| {
            let separation = (v.p - v11).dot(rnormal);
            (separation <= 0.002).then_some(GeometryContact {
                point: v.p - rnormal * (separation * 0.5),
                normal,
                penetration: (-separation).max(0.0),
                feature: base | v.feature,
            })
        })
        .collect()
}

/// Polygon `a` against circle `b`; the normal points from the polygon to the circle.
fn polygon_circle(a: &Body, b: &Body, radius: f32) -> Vec<GeometryContact> {
    let poly = Poly::of(a);
    let c = b.position;
    let mut edge = 0;
    let mut separation = f32::NEG_INFINITY;
    for i in 0..poly.count {
        let s = poly.n[i].dot(c - poly.v[i]);
        if s > radius {
            return vec![];
        }
        if s > separation {
            separation = s;
            edge = i;
        }
    }
    let n = poly.n[edge];
    let face = |separation: f32| GeometryContact {
        point: c - n * separation,
        normal: n,
        penetration: radius - separation,
        feature: edge as u32,
    };
    if separation < 1e-6 {
        return vec![face(separation)];
    }
    let v1 = poly.v[edge];
    let v2 = poly.v[poly.next(edge)];
    let corner = |vertex: Vec2, index: usize| {
        let d = c - vertex;
        let l = d.length();
        (l <= radius).then_some(GeometryContact {
            point: vertex,
            normal: d.normalized(),
            penetration: radius - l,
            feature: 16 + index as u32,
        })
    };
    if (c - v1).dot(v2 - v1) <= 0.0 {
        corner(v1, edge).into_iter().collect()
    } else if (c - v2).dot(v1 - v2) <= 0.0 {
        corner(v2, poly.next(edge)).into_iter().collect()
    } else {
        vec![face(separation)]
    }
}
