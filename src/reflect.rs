use glam::Vec2;

/// A field value that can be read and written at runtime.
///
/// Returned by `Reflect::fields()`, used to edit values in the editor Inspector
/// and reapply them via `Reflect::set_field()`.
///
/// `#[non_exhaustive]`: forks and downstream crates must include a `_` arm when
/// matching this enum (to remain compatible with future variants). Engine-internal
/// variants may be added freely.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ReflectValue {
    F32(f32),
    /// 32-bit integer field (edited as an integer DragValue in the Inspector).
    I32(i32),
    Vec2(Vec2),
    Bool(bool),
    String(String),
    Color([f32; 4]),
}

/// Trait for runtime field read/write.
///
/// Implemented on the built-in engine components (Transform, Sprite, Tag) and
/// can also be implemented manually on user-defined components.
///
/// # egui Inspector integration
/// Registering with `World::register_reflect::<T>()` causes the component's fields
/// to appear automatically in the F1 Inspector panel for live editing.
///
/// # Example
/// ```rust,no_run
/// # use engine::reflect::{Reflect, ReflectValue};
/// struct Hp(f32);
/// impl Reflect for Hp {
///     fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
///         vec![("hp", ReflectValue::F32(self.0))]
///     }
///     fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
///         if name == "hp" { if let ReflectValue::F32(v) = val { self.0 = v; return true; } }
///         false
///     }
///     fn type_name(&self) -> &'static str { "Hp" }
/// }
/// ```
pub trait Reflect {
    /// Returns the current list of field names and values.
    fn fields(&self) -> Vec<(&'static str, ReflectValue)>;
    /// Modifies a field by name. Returns `true` on success.
    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool;
    /// Type name shown in the Inspector.
    fn type_name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A component with an i32 field, to exercise the `ReflectValue::I32` variant.
    struct Score(i32);

    impl Reflect for Score {
        fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
            vec![("score", ReflectValue::I32(self.0))]
        }
        fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
            if name == "score" {
                if let ReflectValue::I32(v) = val {
                    self.0 = v;
                    return true;
                }
            }
            false
        }
        fn type_name(&self) -> &'static str {
            "Score"
        }
    }

    #[test]
    fn reflect_value_i32_roundtrips() {
        let mut s = Score(7);
        assert_eq!(s.fields(), vec![("score", ReflectValue::I32(7))]);
        assert!(s.set_field("score", ReflectValue::I32(42)));
        assert_eq!(s.0, 42);
        // Wrong variant for the field is rejected, value unchanged.
        assert!(!s.set_field("score", ReflectValue::F32(1.0)));
        assert_eq!(s.0, 42);
    }
}
