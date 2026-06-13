//! Procedural macro crate for the `skeleton-engine` Reflect system.
//!
//! Provides `#[derive(Reflect)]` which auto-generates an `engine::reflect::Reflect`
//! implementation for named-field structs whose fields map to supported
//! [`engine::reflect::ReflectValue`] variants (`f32`, `i32`, `Vec2`, `bool`,
//! `String`, `Color`, `[f32; 4]`).
//!
//! Fields annotated with `#[reflect(skip)]` are silently omitted from both
//! `fields()` and `set_field()`.
//!
//! # Example
//!
//! ```rust,ignore
//! use engine::Reflect;
//!
//! #[derive(Reflect)]
//! struct Stats {
//!     hp: f32,
//!     strength: i32,
//! }
//! ```
//!
//! # Unsupported types
//!
//! A field whose type is not one of the recognised variants causes a
//! compile-time error. Use `#[reflect(skip)]` to omit such fields:
//!
//! ```rust,ignore
//! // This DOES NOT compile because `Vec<String>` is not a supported variant.
//! // To fix it, add #[reflect(skip)].
//! use engine::Reflect;
//!
//! #[derive(Reflect)]
//! struct Bad {
//!     tags: Vec<String>, // ERROR: type not supported by #[derive(Reflect)]
//! }
//! ```

mod reflect_derive;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Derives the `engine::reflect::Reflect` trait for a named-field struct.
///
/// Each field whose type maps to a [`engine::reflect::ReflectValue`] variant
/// participates in `fields()` / `set_field()`. Annotate unsupported fields with
/// `#[reflect(skip)]` to exclude them; any other unsupported type is a
/// compile error.
///
/// Supported field types → `ReflectValue` variant:
///
/// | Rust type         | Variant                  |
/// |-------------------|--------------------------|
/// | `f32`             | `ReflectValue::F32`      |
/// | `i32`             | `ReflectValue::I32`      |
/// | `Vec2` / `glam::Vec2` | `ReflectValue::Vec2` |
/// | `bool`            | `ReflectValue::Bool`     |
/// | `String`          | `ReflectValue::String`   |
/// | `Color`           | `ReflectValue::Color`    |
/// | `[f32; 4]`        | `ReflectValue::Color`    |
///
/// # Errors
///
/// - Tuple structs, unit structs, enums, and unions → `compile_error!`
/// - Fields with unsupported types (no `#[reflect(skip)]`) → `compile_error!`
///
/// # Example
///
/// ```rust,ignore
/// use engine::Reflect;
/// use engine::Vec2;
/// use engine::Color;
///
/// #[derive(Reflect)]
/// struct Player {
///     hp: f32,
///     strength: i32,
///     velocity: Vec2,
///     alive: bool,
///     name: String,
///     tint: Color,
///     #[reflect(skip)]
///     internal_id: u64,
/// }
/// ```
#[proc_macro_derive(Reflect, attributes(reflect))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    reflect_derive::expand(input)
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}
