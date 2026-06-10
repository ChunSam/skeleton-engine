//! Behavior Tree system (Phase 36) + Blackboard (Phase 37a)
//!
//! # Core types
//! - [`BehaviorStatus`] — node execution result (`Running` / `Success` / `Failure`)
//! - [`BehaviorNode`] — trait that every node must implement
//! - [`Sequence`] — runs children in order, stops immediately on first Failure
//! - [`Selector`] — runs children in order, stops immediately on first Success
//! - [`Inverter`] — inverts the child result (Success↔Failure)
//! - [`BehaviorTree`] — ECS component. Wraps the root node.
//! - [`BehaviorSystem`] — ticks every entity that has a `BehaviorTree` each frame.
//! - [`Blackboard`] — independent ECS component. Shared key-value state store.
//! - [`BlackboardValue`] — value types that can be stored in a Blackboard.
//!
//! # Example
//! ```rust,no_run
//! use engine::behavior::{BehaviorNode, BehaviorStatus, BehaviorTree, Sequence, Selector, Blackboard};
//! use engine::ecs::World;
//! use engine::System;
//!
//! struct ChasePlayer;
//! impl BehaviorNode for ChasePlayer {
//!     fn tick(&mut self, world: &mut World, entity: engine::ecs::Entity, _dt: f32) -> BehaviorStatus {
//!         // Blackboard is an independent ECS component; access via world.get_mut::<Blackboard>(entity)
//!         if let Some(bb) = world.get_mut::<Blackboard>(entity) {
//!             bb.set_bool("chasing", true);
//!         }
//!         BehaviorStatus::Success
//!     }
//! }
//!
//! let mut world = engine::ecs::World::new();
//! let e = world.spawn();
//! world.add_component(e, BehaviorTree::new(Box::new(Sequence::new(vec![
//!     Box::new(ChasePlayer),
//! ]))));
//! world.add_component(e, Blackboard::new());
//! ```

use std::collections::HashMap;

use glam::{IVec2, Vec2};

use crate::ecs::{Entity, World};
use crate::System;

// ─── Blackboard ───────────────────────────────────────────────────────────────

/// Value types that can be stored in a Blackboard.
///
/// Marked `#[non_exhaustive]`, so external crates must include a wildcard (`_`) arm in `match`.
/// This allows new value types to be added without breaking downstream code.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum BlackboardValue {
    Bool(bool),
    Float(f32),
    Int(i32),
    Vec2(Vec2),
    String(String),
    /// Tile-coordinate path (e.g. A* result). Cache a computed path to avoid recalculating every tick.
    Path(Vec<IVec2>),
}

/// Shared key-value state store used alongside a BehaviorTree.
///
/// An independent ECS component added to the same entity as `BehaviorTree`.
/// Access it inside `BehaviorNode::tick` via `world.get_mut::<Blackboard>(entity)`.
///
/// # Example
/// ```rust,no_run
/// # use engine::behavior::Blackboard;
/// let mut bb = Blackboard::new();
/// bb.set_bool("is_running", true);
/// assert_eq!(bb.get_bool("is_running"), Some(true));
/// assert_eq!(bb.get_bool("unknown"), None);
/// ```
pub struct Blackboard {
    values: HashMap<String, BlackboardValue>,
}

impl Blackboard {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn set_bool(&mut self, key: &str, v: bool) {
        self.values
            .insert(key.to_string(), BlackboardValue::Bool(v));
    }

    pub fn set_float(&mut self, key: &str, v: f32) {
        self.values
            .insert(key.to_string(), BlackboardValue::Float(v));
    }

    pub fn set_int(&mut self, key: &str, v: i32) {
        self.values.insert(key.to_string(), BlackboardValue::Int(v));
    }

    pub fn set_vec2(&mut self, key: &str, v: Vec2) {
        self.values
            .insert(key.to_string(), BlackboardValue::Vec2(v));
    }

    pub fn set_string(&mut self, key: &str, v: impl Into<String>) {
        self.values
            .insert(key.to_string(), BlackboardValue::String(v.into()));
    }

    /// Stores a tile-coordinate path. Cache A* results to avoid recalculating every tick.
    pub fn set_path(&mut self, key: &str, v: Vec<IVec2>) {
        self.values
            .insert(key.to_string(), BlackboardValue::Path(v));
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.values.get(key) {
            Some(BlackboardValue::Bool(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_float(&self, key: &str) -> Option<f32> {
        match self.values.get(key) {
            Some(BlackboardValue::Float(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_int(&self, key: &str) -> Option<i32> {
        match self.values.get(key) {
            Some(BlackboardValue::Int(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_vec2(&self, key: &str) -> Option<Vec2> {
        match self.values.get(key) {
            Some(BlackboardValue::Vec2(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.values.get(key) {
            Some(BlackboardValue::String(v)) => Some(v.as_str()),
            _ => None,
        }
    }

    /// Returns the stored tile-coordinate path as a slice.
    pub fn get_path(&self, key: &str) -> Option<&[IVec2]> {
        match self.values.get(key) {
            Some(BlackboardValue::Path(v)) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns all (key, value) pairs as an iterator. Snapshot use (used by the scripting system).
    pub fn entries(&self) -> impl Iterator<Item = (&str, &BlackboardValue)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v))
    }
}

impl Default for Blackboard {
    fn default() -> Self {
        Self::new()
    }
}

// ─── BehaviorStatus ───────────────────────────────────────────────────────────

/// Behavior node execution result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorStatus {
    /// Still executing (will be ticked again next frame).
    Running,
    /// Completed successfully.
    Success,
    /// Failed.
    Failure,
}

// ─── BehaviorNode trait ───────────────────────────────────────────────────────

/// A single node in a behavior tree.
///
/// Implement this trait to define custom behaviors.
pub trait BehaviorNode: Send + Sync {
    /// Runs for one frame and returns the node's status.
    fn tick(&mut self, world: &mut World, entity: Entity, dt: f32) -> BehaviorStatus;

    /// Resets internal state when the node is restarted (optional).
    fn reset(&mut self) {}
}

// ─── Built-in composite nodes ─────────────────────────────────────────────────

/// Runs child nodes in order.
/// - Child returns `Success` → advance to the next child
/// - Child returns `Running`  → return `Running` (restart the same child next frame)
/// - Child returns `Failure`  → stop immediately and return `Failure`
/// - All children return `Success` → return `Success`
pub struct Sequence {
    children: Vec<Box<dyn BehaviorNode>>,
    current: usize,
}

impl Sequence {
    pub fn new(children: Vec<Box<dyn BehaviorNode>>) -> Self {
        Self {
            children,
            current: 0,
        }
    }
}

impl BehaviorNode for Sequence {
    fn tick(&mut self, world: &mut World, entity: Entity, dt: f32) -> BehaviorStatus {
        while self.current < self.children.len() {
            match self.children[self.current].tick(world, entity, dt) {
                BehaviorStatus::Success => self.current += 1,
                BehaviorStatus::Running => return BehaviorStatus::Running,
                BehaviorStatus::Failure => {
                    self.current = 0;
                    return BehaviorStatus::Failure;
                }
            }
        }
        self.current = 0;
        BehaviorStatus::Success
    }

    fn reset(&mut self) {
        self.current = 0;
        for child in &mut self.children {
            child.reset();
        }
    }
}

/// Runs child nodes in order.
/// - Child returns `Failure`  → advance to the next child
/// - Child returns `Running`  → return `Running`
/// - Child returns `Success`  → stop immediately and return `Success`
/// - All children return `Failure` → return `Failure`
pub struct Selector {
    children: Vec<Box<dyn BehaviorNode>>,
    current: usize,
}

impl Selector {
    pub fn new(children: Vec<Box<dyn BehaviorNode>>) -> Self {
        Self {
            children,
            current: 0,
        }
    }
}

impl BehaviorNode for Selector {
    fn tick(&mut self, world: &mut World, entity: Entity, dt: f32) -> BehaviorStatus {
        while self.current < self.children.len() {
            match self.children[self.current].tick(world, entity, dt) {
                BehaviorStatus::Failure => self.current += 1,
                BehaviorStatus::Running => return BehaviorStatus::Running,
                BehaviorStatus::Success => {
                    self.current = 0;
                    return BehaviorStatus::Success;
                }
            }
        }
        self.current = 0;
        BehaviorStatus::Failure
    }

    fn reset(&mut self) {
        self.current = 0;
        for child in &mut self.children {
            child.reset();
        }
    }
}

/// Inverts the child node's result (Success → Failure, Failure → Success, Running unchanged).
pub struct Inverter {
    child: Box<dyn BehaviorNode>,
}

impl Inverter {
    pub fn new(child: Box<dyn BehaviorNode>) -> Self {
        Self { child }
    }
}

impl BehaviorNode for Inverter {
    fn tick(&mut self, world: &mut World, entity: Entity, dt: f32) -> BehaviorStatus {
        match self.child.tick(world, entity, dt) {
            BehaviorStatus::Success => BehaviorStatus::Failure,
            BehaviorStatus::Failure => BehaviorStatus::Success,
            BehaviorStatus::Running => BehaviorStatus::Running,
        }
    }

    fn reset(&mut self) {
        self.child.reset();
    }
}

/// Decorator node that always returns `Success`.
pub struct AlwaysSucceed {
    child: Box<dyn BehaviorNode>,
}

impl AlwaysSucceed {
    pub fn new(child: Box<dyn BehaviorNode>) -> Self {
        Self { child }
    }
}

impl BehaviorNode for AlwaysSucceed {
    fn tick(&mut self, world: &mut World, entity: Entity, dt: f32) -> BehaviorStatus {
        self.child.tick(world, entity, dt);
        BehaviorStatus::Success
    }

    fn reset(&mut self) {
        self.child.reset();
    }
}

// ─── BehaviorTree component ───────────────────────────────────────────────────

/// ECS component. Wraps the root `BehaviorNode`.
///
/// `BehaviorSystem` calls `tick()` every frame.
/// Because `BehaviorNode: Send + Sync`, `BehaviorTree` is also thread-safe.
pub struct BehaviorTree {
    root: Box<dyn BehaviorNode>,
}

impl BehaviorTree {
    pub fn new(root: Box<dyn BehaviorNode>) -> Self {
        Self { root }
    }

    pub fn tick(&mut self, world: &mut World, entity: Entity, dt: f32) -> BehaviorStatus {
        self.root.tick(world, entity, dt)
    }

    /// Resets the entire tree state starting from the root.
    pub fn reset(&mut self) {
        self.root.reset();
    }
}

// ─── BehaviorSystem ───────────────────────────────────────────────────────────

/// System that ticks every entity with a `BehaviorTree` component each frame.
///
/// # Registration
/// ```rust,no_run
/// # use engine::App;
/// # use engine::behavior::BehaviorSystem;
/// let mut app = App::new();
/// app.add_system(BehaviorSystem);
/// ```
pub struct BehaviorSystem;

impl BehaviorSystem {
    /// Schedule label for ordering via `add_system_labeled`.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::behavior";
}

impl System for BehaviorSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // Borrow-checker workaround: collect entity list first.
        let entities: Vec<Entity> = world.query::<BehaviorTree>().map(|(e, _)| e).collect();

        for entity in entities {
            // Temporarily remove BehaviorTree, tick it, then put it back.
            // take_component → tick(world) → add_component avoids a double borrow.
            if let Some(mut tree) = world.take_component::<BehaviorTree>(entity) {
                tree.tick(world, entity, dt);
                world.add_component(entity, tree);
            }
        }
    }

    fn name(&self) -> &'static str {
        "BehaviorSystem"
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysOk;
    impl BehaviorNode for AlwaysOk {
        fn tick(&mut self, _: &mut World, _: Entity, _: f32) -> BehaviorStatus {
            BehaviorStatus::Success
        }
    }

    struct AlwaysFail;
    impl BehaviorNode for AlwaysFail {
        fn tick(&mut self, _: &mut World, _: Entity, _: f32) -> BehaviorStatus {
            BehaviorStatus::Failure
        }
    }

    struct AlwaysRun;
    impl BehaviorNode for AlwaysRun {
        fn tick(&mut self, _: &mut World, _: Entity, _: f32) -> BehaviorStatus {
            BehaviorStatus::Running
        }
    }

    fn dummy() -> (World, Entity) {
        let mut w = World::new();
        let e = w.spawn();
        (w, e)
    }

    #[test]
    fn blackboard_path_roundtrip() {
        let mut bb = Blackboard::new();
        let path = vec![IVec2::new(1, 2), IVec2::new(3, 4), IVec2::new(5, 6)];
        bb.set_path("route", path.clone());
        assert_eq!(bb.get_path("route"), Some(path.as_slice()));
        // Type mismatch / missing key returns None.
        assert_eq!(bb.get_path("missing"), None);
        bb.set_vec2("v", Vec2::new(1.0, 2.0));
        assert_eq!(bb.get_path("v"), None);
        assert_eq!(bb.get_vec2("route"), None);
    }

    #[test]
    fn sequence_all_success() {
        let (mut w, e) = dummy();
        let mut seq = Sequence::new(vec![Box::new(AlwaysOk), Box::new(AlwaysOk)]);
        assert_eq!(seq.tick(&mut w, e, 0.016), BehaviorStatus::Success);
    }

    #[test]
    fn sequence_fails_on_failure() {
        let (mut w, e) = dummy();
        let mut seq = Sequence::new(vec![
            Box::new(AlwaysOk),
            Box::new(AlwaysFail),
            Box::new(AlwaysOk),
        ]);
        assert_eq!(seq.tick(&mut w, e, 0.016), BehaviorStatus::Failure);
    }

    #[test]
    fn sequence_running_pauses() {
        let (mut w, e) = dummy();
        let mut seq = Sequence::new(vec![
            Box::new(AlwaysOk),
            Box::new(AlwaysRun),
            Box::new(AlwaysOk),
        ]);
        assert_eq!(seq.tick(&mut w, e, 0.016), BehaviorStatus::Running);
    }

    #[test]
    fn selector_succeeds_on_first_success() {
        let (mut w, e) = dummy();
        let mut sel = Selector::new(vec![
            Box::new(AlwaysFail),
            Box::new(AlwaysOk),
            Box::new(AlwaysFail),
        ]);
        assert_eq!(sel.tick(&mut w, e, 0.016), BehaviorStatus::Success);
    }

    #[test]
    fn selector_fails_when_all_fail() {
        let (mut w, e) = dummy();
        let mut sel = Selector::new(vec![Box::new(AlwaysFail), Box::new(AlwaysFail)]);
        assert_eq!(sel.tick(&mut w, e, 0.016), BehaviorStatus::Failure);
    }

    #[test]
    fn inverter_flips_success() {
        let (mut w, e) = dummy();
        let mut inv = Inverter::new(Box::new(AlwaysOk));
        assert_eq!(inv.tick(&mut w, e, 0.016), BehaviorStatus::Failure);
    }

    #[test]
    fn inverter_flips_failure() {
        let (mut w, e) = dummy();
        let mut inv = Inverter::new(Box::new(AlwaysFail));
        assert_eq!(inv.tick(&mut w, e, 0.016), BehaviorStatus::Success);
    }

    #[test]
    fn behavior_system_ticks_entity() {
        use std::sync::{Arc, Mutex};

        let ticked = Arc::new(Mutex::new(false));
        let ticked_clone = Arc::clone(&ticked);

        struct TickRecorder(Arc<Mutex<bool>>);
        impl BehaviorNode for TickRecorder {
            fn tick(&mut self, _: &mut World, _: Entity, _: f32) -> BehaviorStatus {
                *self.0.lock().unwrap() = true;
                BehaviorStatus::Success
            }
        }

        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, BehaviorTree::new(Box::new(TickRecorder(ticked_clone))));

        let mut sys = BehaviorSystem;
        sys.run(&mut world, 0.016);

        assert!(*ticked.lock().unwrap());
    }

    // ── Blackboard tests ──────────────────────────────────────────────────────

    #[test]
    fn blackboard_bool() {
        let mut bb = Blackboard::new();
        assert_eq!(bb.get_bool("flag"), None);
        bb.set_bool("flag", true);
        assert_eq!(bb.get_bool("flag"), Some(true));
        bb.set_bool("flag", false);
        assert_eq!(bb.get_bool("flag"), Some(false));
    }

    #[test]
    fn blackboard_float() {
        let mut bb = Blackboard::new();
        assert_eq!(bb.get_float("speed"), None);
        bb.set_float("speed", 3.125);
        let v = bb.get_float("speed").unwrap();
        assert!((v - 3.125).abs() < 1e-5);
    }

    #[test]
    fn blackboard_int() {
        let mut bb = Blackboard::new();
        assert_eq!(bb.get_int("count"), None);
        bb.set_int("count", 42);
        assert_eq!(bb.get_int("count"), Some(42));
    }

    #[test]
    fn blackboard_vec2() {
        let mut bb = Blackboard::new();
        assert!(bb.get_vec2("pos").is_none());
        bb.set_vec2("pos", Vec2::new(1.0, 2.0));
        let v = bb.get_vec2("pos").unwrap();
        assert!((v.x - 1.0).abs() < 1e-5);
        assert!((v.y - 2.0).abs() < 1e-5);
    }

    #[test]
    fn blackboard_string() {
        let mut bb = Blackboard::new();
        assert!(bb.get_string("name").is_none());
        bb.set_string("name", "hero");
        assert_eq!(bb.get_string("name"), Some("hero"));
    }

    #[test]
    fn blackboard_missing_key_returns_none() {
        let bb = Blackboard::new();
        assert!(bb.get_bool("x").is_none());
        assert!(bb.get_float("x").is_none());
        assert!(bb.get_int("x").is_none());
        assert!(bb.get_vec2("x").is_none());
        assert!(bb.get_string("x").is_none());
    }
}
