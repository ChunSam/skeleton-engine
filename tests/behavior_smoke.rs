//! Smoke tests for the behavior-tree module.
//!
//! Pure logic — no window, GPU, or event-loop required.
//! Exercises: `BehaviorNode`, `BehaviorStatus`, `Sequence`, `Selector`,
//! `Inverter`, `AlwaysSucceed`, `BehaviorTree`, `Blackboard`, `BlackboardValue`.

use engine::{
    AlwaysSucceed, BehaviorNode, BehaviorStatus, BehaviorTree, Blackboard, Entity, Inverter,
    Selector, Sequence, World,
};

// ── Minimal leaf nodes ────────────────────────────────────────────────────────

struct Succeed;
impl BehaviorNode for Succeed {
    fn tick(&mut self, _world: &mut World, _entity: Entity, _dt: f32) -> BehaviorStatus {
        BehaviorStatus::Success
    }
}

struct Fail;
impl BehaviorNode for Fail {
    fn tick(&mut self, _world: &mut World, _entity: Entity, _dt: f32) -> BehaviorStatus {
        BehaviorStatus::Failure
    }
}

struct Running;
impl BehaviorNode for Running {
    fn tick(&mut self, _world: &mut World, _entity: Entity, _dt: f32) -> BehaviorStatus {
        BehaviorStatus::Running
    }
}

// Helper: tick a tree once against a fresh World.
fn tick_once(mut tree: BehaviorTree) -> BehaviorStatus {
    let mut world = World::new();
    let entity = world.spawn();
    tree.tick(&mut world, entity, 0.016)
}

// ── Sequence ──────────────────────────────────────────────────────────────────

/// Sequence of all-success nodes returns Success.
#[test]
fn sequence_all_succeed() {
    let tree = BehaviorTree::new(Box::new(Sequence::new(vec![
        Box::new(Succeed),
        Box::new(Succeed),
    ])));
    assert_eq!(tick_once(tree), BehaviorStatus::Success);
}

/// Sequence aborts on first Failure.
#[test]
fn sequence_first_failure_aborts() {
    let tree = BehaviorTree::new(Box::new(Sequence::new(vec![
        Box::new(Fail),
        Box::new(Succeed), // should not be reached
    ])));
    assert_eq!(tick_once(tree), BehaviorStatus::Failure);
}

/// Sequence propagates Running.
#[test]
fn sequence_running_propagates() {
    let tree = BehaviorTree::new(Box::new(Sequence::new(vec![
        Box::new(Running),
        Box::new(Succeed),
    ])));
    assert_eq!(tick_once(tree), BehaviorStatus::Running);
}

// ── Selector ──────────────────────────────────────────────────────────────────

/// Selector returns Success on first succeeding child.
#[test]
fn selector_first_success() {
    let tree = BehaviorTree::new(Box::new(Selector::new(vec![
        Box::new(Fail),
        Box::new(Succeed),
        Box::new(Fail),
    ])));
    assert_eq!(tick_once(tree), BehaviorStatus::Success);
}

/// Selector returns Failure when all children fail.
#[test]
fn selector_all_fail() {
    let tree = BehaviorTree::new(Box::new(Selector::new(vec![
        Box::new(Fail),
        Box::new(Fail),
    ])));
    assert_eq!(tick_once(tree), BehaviorStatus::Failure);
}

// ── Inverter ──────────────────────────────────────────────────────────────────

/// Inverter flips Success → Failure.
#[test]
fn inverter_flips_success() {
    let tree = BehaviorTree::new(Box::new(Inverter::new(Box::new(Succeed))));
    assert_eq!(tick_once(tree), BehaviorStatus::Failure);
}

/// Inverter flips Failure → Success.
#[test]
fn inverter_flips_failure() {
    let tree = BehaviorTree::new(Box::new(Inverter::new(Box::new(Fail))));
    assert_eq!(tick_once(tree), BehaviorStatus::Success);
}

/// Inverter passes Running through unchanged.
#[test]
fn inverter_passes_running() {
    let tree = BehaviorTree::new(Box::new(Inverter::new(Box::new(Running))));
    assert_eq!(tick_once(tree), BehaviorStatus::Running);
}

// ── AlwaysSucceed ─────────────────────────────────────────────────────────────

/// AlwaysSucceed converts Failure to Success.
#[test]
fn always_succeed_converts_failure() {
    let tree = BehaviorTree::new(Box::new(AlwaysSucceed::new(Box::new(Fail))));
    assert_eq!(tick_once(tree), BehaviorStatus::Success);
}

/// AlwaysSucceed passes Running through.
#[test]
fn always_succeed_passes_running() {
    let tree = BehaviorTree::new(Box::new(AlwaysSucceed::new(Box::new(Running))));
    assert_eq!(tick_once(tree), BehaviorStatus::Running);
}

// ── Blackboard ────────────────────────────────────────────────────────────────

/// Blackboard stores and retrieves typed values.
#[test]
fn blackboard_typed_values() {
    let mut bb = Blackboard::new();

    bb.set_bool("alive", true);
    assert_eq!(bb.get_bool("alive"), Some(true));
    assert_eq!(bb.get_bool("missing"), None);

    bb.set_float("speed", 2.5);
    let speed = bb.get_float("speed").unwrap();
    assert!((speed - 2.5).abs() < 1e-6);

    bb.set_int("score", 42);
    assert_eq!(bb.get_int("score"), Some(42));

    bb.set_string("name", "hero");
    assert_eq!(bb.get_string("name"), Some("hero"));
}

/// Blackboard path round-trip via set_path / get_path.
#[test]
fn blackboard_path_round_trip() {
    use engine::IVec2;
    let mut bb = Blackboard::new();
    let path = vec![IVec2::new(0, 0), IVec2::new(1, 0), IVec2::new(1, 1)];
    bb.set_path("route", path.clone());
    let stored = bb.get_path("route").unwrap();
    assert_eq!(stored, path.as_slice());
}
