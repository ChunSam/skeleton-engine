//! End-to-end integration test for `#[derive(Reflect)]`.
//!
//! Placed in the ENGINE crate's `tests/` directory (not in the proc-macro
//! crate) to avoid a dependency cycle: the proc-macro crate cannot depend on
//! `engine` (engine depends on the macro), so any test that needs both the
//! `Reflect` trait AND the derive macro must live here, where `engine` is the
//! crate under test.

#![cfg(feature = "derive")]

use engine::reflect::{Reflect, ReflectValue};
use engine::Color;
use engine::Vec2;

/// Test struct exercising all supported field types plus `#[reflect(skip)]`.
#[derive(engine::Reflect)]
struct Stats {
    hp: f32,
    strength: i32,
    velocity: Vec2,
    alive: bool,
    name: String,
    tint: Color,
    /// This field is deliberately unsupported (`u64`) but excluded via skip.
    #[reflect(skip)]
    internal_id: u64,
}

#[test]
fn fields_returns_correct_count_and_values() {
    let s = Stats {
        hp: 100.0,
        strength: 5,
        velocity: Vec2::new(1.0, 2.0),
        alive: true,
        name: "Hero".to_string(),
        tint: Color::rgb(1.0, 0.0, 0.0),
        internal_id: 42,
    };

    let fields = s.fields();
    // 6 supported fields; internal_id is skipped.
    assert_eq!(fields.len(), 6, "expected 6 fields (internal_id skipped)");

    assert_eq!(fields[0], ("hp", ReflectValue::F32(100.0)));
    assert_eq!(fields[1], ("strength", ReflectValue::I32(5)));
    assert_eq!(
        fields[2],
        ("velocity", ReflectValue::Vec2(Vec2::new(1.0, 2.0)))
    );
    assert_eq!(fields[3], ("alive", ReflectValue::Bool(true)));
    assert_eq!(
        fields[4],
        ("name", ReflectValue::String("Hero".to_string()))
    );
    assert_eq!(
        fields[5],
        ("tint", ReflectValue::Color([1.0, 0.0, 0.0, 1.0]))
    );
}

#[test]
fn set_field_correct_variant_succeeds() {
    let mut s = Stats {
        hp: 0.0,
        strength: 0,
        velocity: Vec2::ZERO,
        alive: false,
        name: String::new(),
        tint: Color::WHITE,
        internal_id: 0,
    };

    assert!(s.set_field("hp", ReflectValue::F32(50.0)));
    assert_eq!(s.hp, 50.0);

    assert!(s.set_field("strength", ReflectValue::I32(9)));
    assert_eq!(s.strength, 9);

    assert!(s.set_field("velocity", ReflectValue::Vec2(Vec2::new(3.0, 4.0))));
    assert_eq!(s.velocity, Vec2::new(3.0, 4.0));

    assert!(s.set_field("alive", ReflectValue::Bool(true)));
    assert!(s.alive);

    assert!(s.set_field("name", ReflectValue::String("Warrior".to_string())));
    assert_eq!(s.name, "Warrior");

    assert!(s.set_field("tint", ReflectValue::Color([0.0, 1.0, 0.0, 1.0])));
    assert_eq!(s.tint.to_array(), [0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn set_field_wrong_variant_returns_false() {
    let mut s = Stats {
        hp: 10.0,
        strength: 1,
        velocity: Vec2::ZERO,
        alive: true,
        name: "X".to_string(),
        tint: Color::WHITE,
        internal_id: 0,
    };

    // Wrong variant for hp (I32 instead of F32) — must return false, value unchanged.
    assert!(!s.set_field("hp", ReflectValue::I32(99)));
    assert_eq!(s.hp, 10.0);
}

#[test]
fn set_field_unknown_field_returns_false() {
    let mut s = Stats {
        hp: 10.0,
        strength: 1,
        velocity: Vec2::ZERO,
        alive: true,
        name: "X".to_string(),
        tint: Color::WHITE,
        internal_id: 0,
    };

    assert!(!s.set_field("nonexistent", ReflectValue::F32(0.0)));
}

#[test]
fn set_field_skipped_field_returns_false() {
    let mut s = Stats {
        hp: 10.0,
        strength: 1,
        velocity: Vec2::ZERO,
        alive: true,
        name: "X".to_string(),
        tint: Color::WHITE,
        internal_id: 99,
    };

    // internal_id is skipped — set_field must not touch it.
    assert!(!s.set_field("internal_id", ReflectValue::I32(0)));
    assert_eq!(s.internal_id, 99);
}

#[test]
fn type_name_returns_struct_name() {
    let s = Stats {
        hp: 0.0,
        strength: 0,
        velocity: Vec2::ZERO,
        alive: false,
        name: String::new(),
        tint: Color::WHITE,
        internal_id: 0,
    };
    assert_eq!(s.type_name(), "Stats");
}

/// Test `[f32; 4]` field (direct array, no Color wrapper).
#[derive(engine::Reflect)]
struct RawColor {
    rgba: [f32; 4],
}

#[test]
fn array4_field_maps_to_color_variant() {
    let rc = RawColor {
        rgba: [0.1, 0.2, 0.3, 0.4],
    };
    let fields = rc.fields();
    assert_eq!(fields.len(), 1);
    assert_eq!(
        fields[0],
        ("rgba", ReflectValue::Color([0.1, 0.2, 0.3, 0.4]))
    );
}

#[test]
fn array4_set_field_roundtrips() {
    let mut rc = RawColor { rgba: [0.0; 4] };
    assert!(rc.set_field("rgba", ReflectValue::Color([1.0, 0.5, 0.25, 0.75])));
    assert_eq!(rc.rgba, [1.0, 0.5, 0.25, 0.75]);
}
