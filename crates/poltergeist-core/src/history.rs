//! A bounded timeline of recent world states for scrubbing back through time.
//!
//! Each tick records every body's motion state. Any structural change (adding or
//! removing a body or joint) starts a fresh timeline, because older states no
//! longer describe the same set of bodies. Contact caches are not recorded, so a
//! replay resumed from a rewound state can diverge slightly from the original run.
use crate::{Vec2, World};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyState {
    pub position: Vec2,
    pub angle: f32,
    pub velocity: Vec2,
    pub angular_velocity: f32,
    pub awake: bool,
    pub sleep_time: f32,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub time: f32,
    pub tick: u64,
    pub(crate) version: u64,
    pub(crate) bodies: Vec<BodyState>,
}

pub struct History {
    states: VecDeque<Snapshot>,
    cursor: usize,
    capacity: usize,
}
impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            states: VecDeque::new(),
            cursor: 0,
            capacity: capacity.max(1),
        }
    }
    pub fn len(&self) -> usize {
        self.states.len()
    }
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
    /// Index of the state the world currently shows.
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn clear(&mut self) {
        self.states.clear();
        self.cursor = 0;
    }
    /// Appends the world's state after a step. Any rewound future is discarded first.
    pub fn record(&mut self, world: &World) {
        if self
            .states
            .back()
            .is_some_and(|s| s.version != world.structure_version())
        {
            self.clear();
        }
        self.states.truncate(self.cursor + 1);
        let reuse = if self.states.len() >= self.capacity {
            self.states.pop_front().map(|s| s.bodies)
        } else {
            None
        };
        self.states
            .push_back(world.capture(reuse.unwrap_or_default()));
        self.cursor = self.states.len() - 1;
    }
    /// Re-anchors the timeline after an edit made outside a step, so the present
    /// reflects the edit and any rewound future is discarded.
    pub fn sync(&mut self, world: &World) {
        let current = self.states.get(self.cursor);
        if current.is_none_or(|s| s.version != world.structure_version()) {
            self.clear();
            self.states.push_back(world.capture(Vec::new()));
            return;
        }
        self.states.truncate(self.cursor + 1);
        let recycled = std::mem::take(&mut self.states[self.cursor].bodies);
        self.states[self.cursor] = world.capture(recycled);
    }
    /// Restores the world to the recorded state at `index` (0 is the oldest).
    pub fn seek(&mut self, world: &mut World, index: usize) -> bool {
        match self.states.get(index) {
            Some(s) if s.version == world.structure_version() => {
                world.restore(s);
                self.cursor = index;
                true
            }
            _ => false,
        }
    }
}
