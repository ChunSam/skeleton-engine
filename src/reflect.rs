use glam::Vec2;

/// 런타임에 읽고 쓸 수 있는 필드 값.
///
/// `Reflect::fields()`에서 반환되며, 에디터 Inspector에서 값을 편집하고
/// `Reflect::set_field()`로 다시 적용할 때 사용한다.
///
/// `#[non_exhaustive]`: 포크/다운스트림에서 이 enum 을 `match` 할 때는 `_` 분기를
/// 둬야 한다 (향후 변형 추가에 대비). 엔진 내부 변형은 자유롭게 추가될 수 있다.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ReflectValue {
    F32(f32),
    /// 32비트 정수 필드 (Inspector 에서 정수 DragValue 로 편집).
    I32(i32),
    Vec2(Vec2),
    Bool(bool),
    String(String),
    Color([f32; 4]),
}

/// 런타임 필드 읽기/쓰기 트레잇.
///
/// 엔진 내장 컴포넌트(Transform, Sprite, Tag)에 구현되어 있으며,
/// 사용자 컴포넌트에도 수동으로 구현할 수 있다.
///
/// # egui Inspector 연동
/// `World::register_reflect::<T>()` 로 등록하면 F1 Inspector 패널에
/// 해당 컴포넌트의 필드가 자동으로 표시되어 실시간 편집이 가능하다.
///
/// # 예시
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
    /// 현재 필드 이름·값 목록을 반환한다.
    fn fields(&self) -> Vec<(&'static str, ReflectValue)>;
    /// 이름으로 필드를 수정한다. 성공하면 `true` 반환.
    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool;
    /// Inspector 표시용 타입 이름.
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
