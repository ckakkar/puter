//! Preset experiments. Body IDs follow creation order, so keep existing scenes stable.
use crate::{JointKind, Shape, Vec2, World};

pub const SCENE_COUNT: u32 = 8;

impl World {
    pub fn load_scene(&mut self, scene: u32) {
        self.clear();
        self.add(
            Shape::Box {
                half: Vec2::new(9.0, 0.3),
            },
            Vec2::new(0.0, -0.3),
            0.0,
        );
        match scene {
            1 => self.pendulums(),
            2 => self.friction_study(),
            3 => self.bounce_chamber(),
            4 => self.domino_run(),
            5 => self.spring_lattices(),
            6 => self.mechanisms(),
            7 => {}
            _ => self.tower(),
        }
        // Side walls keep normal-speed interactions within the laboratory.
        for x in [-9.25, 9.25] {
            self.add(
                Shape::Box {
                    half: Vec2::new(0.25, 4.5),
                },
                Vec2::new(x, 4.2),
                0.0,
            );
        }
        self.refresh_contacts();
    }
    fn set(&mut self, id: u32, angle: f32, friction: f32, restitution: f32) {
        if let Some(b) = self.body_mut(id) {
            b.angle = angle;
            b.friction = friction;
            b.restitution = restitution;
        }
    }
    fn tower(&mut self) {
        for row in 0..7 {
            for col in 0..3 {
                let id = self.add(
                    Shape::Box {
                        half: Vec2::new(0.48, 0.38),
                    },
                    Vec2::new((col as f32 - 1.0) * 1.0, row as f32 * 0.78 + 0.4),
                    1.0,
                );
                self.set(id, 0.0, 0.55, 0.0);
            }
        }
        let id = self.add(Shape::Circle { radius: 0.6 }, Vec2::new(-5.0, 3.0), 3.0);
        self.set(id, 0.0, 0.55, 0.45);
    }
    fn pendulums(&mut self) {
        for i in 0..5 {
            let x = -4.0 + i as f32 * 2.0;
            let anchor = self.add(Shape::Circle { radius: 0.11 }, Vec2::new(x, 6.7), 0.0);
            let bob = self.add(Shape::Circle { radius: 0.42 }, Vec2::new(x + 0.6, 3.2), 2.0);
            self.add_joint(
                JointKind::Rod,
                anchor,
                bob,
                Vec2::new(x, 6.7),
                Vec2::new(x + 0.6, 3.2),
            );
        }
    }
    fn friction_study(&mut self) {
        for (i, friction) in [0.02, 0.2, 0.9].into_iter().enumerate() {
            let y = 1.3 + i as f32 * 1.8;
            let angle = -0.14;
            let id = self.add(
                Shape::Box {
                    half: Vec2::new(5.8, 0.12),
                },
                Vec2::new(0.0, y),
                0.0,
            );
            self.set(id, angle, friction, 0.12);
            let pos = Vec2::new(-4.5, 0.55).rotate(angle) + Vec2::new(0.0, y);
            let block = self.add(
                Shape::Box {
                    half: Vec2::new(0.45, 0.4),
                },
                pos,
                1.0,
            );
            self.set(block, angle, friction, 0.12);
        }
    }
    fn bounce_chamber(&mut self) {
        for i in 0..7 {
            let id = self.add(
                Shape::Circle {
                    radius: 0.3 + (i % 3) as f32 * 0.08,
                },
                Vec2::new(-4.5 + i as f32 * 1.4, 2.5 + (i % 2) as f32 * 2.0),
                1.0,
            );
            self.set(id, 0.0, 0.55, 0.72);
        }
        let id = self.add(
            Shape::Box {
                half: Vec2::new(1.6, 0.15),
            },
            Vec2::new(0.0, 2.0),
            0.0,
        );
        self.set(id, 0.25, 0.55, 0.12);
    }
    fn domino_run(&mut self) {
        let ramp = self.add(
            Shape::Box {
                half: Vec2::new(1.5, 0.08),
            },
            Vec2::new(-7.4, 1.6),
            0.0,
        );
        self.set(ramp, -0.35, 0.6, 0.0);
        let ball = self.add(Shape::Circle { radius: 0.3 }, Vec2::new(-8.3, 2.55), 1.5);
        self.set(ball, 0.0, 0.6, 0.1);
        for i in 0..18 {
            let id = self.add(
                Shape::Box {
                    half: Vec2::new(0.07, 0.55),
                },
                Vec2::new(-5.55 + i as f32 * 0.62, 0.55),
                0.4,
            );
            self.set(id, 0.0, 0.5, 0.0);
        }
        let stop = self.add(
            Shape::Polygon {
                radius: 0.5,
                sides: 3,
            },
            Vec2::new(6.4, 0.25),
            0.0,
        );
        self.set(stop, 0.0, 0.6, 0.0);
    }
    fn spring_lattices(&mut self) {
        for (origin, frequency) in [(Vec2::new(-5.8, 2.4), 3.5), (Vec2::new(2.2, 2.4), 8.0)] {
            let mut grid = [[0u32; 4]; 4];
            for (row, cells) in grid.iter_mut().enumerate() {
                for (col, cell) in cells.iter_mut().enumerate() {
                    let p = origin + Vec2::new(col as f32 * 0.62, row as f32 * 0.62);
                    *cell = self.add(Shape::Circle { radius: 0.2 }, p, 0.3);
                    self.set(*cell, 0.0, 0.5, 0.05);
                }
            }
            let kind = JointKind::Spring {
                frequency,
                damping: 0.25,
            };
            let link = |w: &mut World, a: u32, b: u32| {
                let (pa, pb) = (w.body(a).unwrap().position, w.body(b).unwrap().position);
                w.add_joint(kind, a, b, pa, pb);
            };
            for row in 0..4 {
                for col in 0..4 {
                    if col < 3 {
                        link(self, grid[row][col], grid[row][col + 1]);
                    }
                    if row < 3 {
                        link(self, grid[row][col], grid[row + 1][col]);
                    }
                    if row < 3 && col < 3 {
                        link(self, grid[row][col], grid[row + 1][col + 1]);
                        link(self, grid[row][col + 1], grid[row + 1][col]);
                    }
                }
            }
            let weight = self.add(
                Shape::Box {
                    half: Vec2::new(0.4, 0.3),
                },
                origin + Vec2::new(0.93, 3.8),
                3.0,
            );
            self.set(weight, 0.0, 0.5, 0.0);
        }
    }
    fn mechanisms(&mut self) {
        // A motor-driven paddle pinned to the world at its center.
        let paddle_at = Vec2::new(-4.4, 3.1);
        let paddle = self.add(
            Shape::Box {
                half: Vec2::new(1.9, 0.09),
            },
            paddle_at,
            2.0,
        );
        self.add_joint(
            JointKind::Pin {
                motor_speed: -1.4,
                max_torque: 250.0,
            },
            paddle,
            0,
            paddle_at,
            paddle_at,
        );
        for i in 0..6 {
            let id = self.add(
                Shape::Circle { radius: 0.22 },
                Vec2::new(-5.9 + i as f32 * 0.6, 6.2 + (i % 2) as f32 * 0.7),
                0.4,
            );
            self.set(id, 0.0, 0.4, 0.4);
        }
        // A see-saw: a plank pinned to the apex of a static triangle.
        let fulcrum = self.add(
            Shape::Polygon {
                radius: 0.6,
                sides: 3,
            },
            Vec2::new(4.6, 0.3),
            0.0,
        );
        self.set(fulcrum, 0.0, 0.6, 0.0);
        let pivot = Vec2::new(4.6, 0.98);
        let plank = self.add(
            Shape::Box {
                half: Vec2::new(2.6, 0.08),
            },
            pivot,
            1.5,
        );
        self.add_joint(
            JointKind::Pin {
                motor_speed: 0.0,
                max_torque: 0.0,
            },
            plank,
            fulcrum,
            pivot,
            pivot,
        );
        let ball = self.add(Shape::Circle { radius: 0.25 }, Vec2::new(2.3, 1.4), 0.3);
        self.set(ball, 0.0, 0.6, 0.2);
        let weight = self.add(
            Shape::Box {
                half: Vec2::new(0.35, 0.35),
            },
            Vec2::new(6.8, 6.0),
            5.0,
        );
        self.set(weight, 0.0, 0.6, 0.0);
        // A rope pendulum and a spring hanging from the ceiling.
        let rope_anchor = Vec2::new(1.2, 7.8);
        let bob = self.add(Shape::Circle { radius: 0.3 }, Vec2::new(3.4, 7.2), 1.0);
        self.set(bob, 0.0, 0.5, 0.3);
        self.add_joint(JointKind::Rope, bob, 0, Vec2::new(3.4, 7.2), rope_anchor);
        let spring_anchor = Vec2::new(-0.6, 7.8);
        let hanging = self.add(
            Shape::Polygon {
                radius: 0.36,
                sides: 6,
            },
            Vec2::new(-0.6, 4.6),
            1.0,
        );
        self.add_joint(
            JointKind::Spring {
                frequency: 1.1,
                damping: 0.05,
            },
            hanging,
            0,
            Vec2::new(-0.6, 4.6) + Vec2::new(0.0, 0.31),
            spring_anchor,
        );
        if let Some(j) = self.joints.last_mut() {
            j.length = 2.2;
        }
    }
}
