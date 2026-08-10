//! Dialogue variables, conditions, and effects — the data behind gated / consequential choices.
//!
//! A [`DialogueChoice`](super::DialogueChoice) may carry:
//! - a [`DialogueCond`] — the choice is only shown ([`DialogueBox::visible_choices`](super::DialogueBox::visible_choices))
//!   when the condition passes against the [`DialogueVars`] resource, and
//! - a [`DialogueEffect`] — applied when the choice is taken: set a variable, or emit a
//!   [`DialogueEvent`] the game reads from `Events<DialogueEvent>`.
//!
//! Variables live in the [`DialogueVars`] World resource (game-owned: insert it and seed
//! starting flags). The world-level [`dialogue::advance`](super::advance) /
//! [`dialogue::choose`](super::choose) helpers read it to filter choices and write it when a
//! chosen choice has a `SetVar` effect.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ecs::Entity;

/// A small serde-friendly value for dialogue variables, conditions, and effects.
///
/// `Eq` is intentionally not derived (the `Float` variant holds an `f32`); compare with
/// [`DialogueCond`] instead of direct equality where ordering matters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DialogueValue {
    /// A boolean flag.
    Bool(bool),
    /// An integer counter.
    Int(i64),
    /// A floating-point number.
    Float(f32),
    /// A string value.
    Str(String),
}

impl DialogueValue {
    /// The zero/empty value of this value's variant (used as the stand-in for an unset
    /// variable so e.g. `flag == false` is true when `flag` was never set).
    fn zero_like(&self) -> DialogueValue {
        match self {
            DialogueValue::Bool(_) => DialogueValue::Bool(false),
            DialogueValue::Int(_) => DialogueValue::Int(0),
            DialogueValue::Float(_) => DialogueValue::Float(0.0),
            DialogueValue::Str(_) => DialogueValue::Str(String::new()),
        }
    }

    /// Numeric view for ordered comparisons (`Int`/`Float` only).
    fn as_f64(&self) -> Option<f64> {
        match self {
            DialogueValue::Int(i) => Some(*i as f64),
            DialogueValue::Float(f) => Some(*f as f64),
            _ => None,
        }
    }

    /// Returns the bool if this is a [`DialogueValue::Bool`].
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DialogueValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the int if this is a [`DialogueValue::Int`].
    pub fn as_int(&self) -> Option<i64> {
        match self {
            DialogueValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the float if this is a [`DialogueValue::Float`].
    pub fn as_float(&self) -> Option<f32> {
        match self {
            DialogueValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns the string if this is a [`DialogueValue::Str`].
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DialogueValue::Str(s) => Some(s),
            _ => None,
        }
    }
}

/// Comparison operator for a [`DialogueCond`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogueOp {
    /// Equal.
    Eq,
    /// Not equal.
    Ne,
    /// Greater than (numeric).
    Gt,
    /// Less than (numeric).
    Lt,
    /// Greater than or equal (numeric).
    Ge,
    /// Less than or equal (numeric).
    Le,
}

/// A condition gating a [`DialogueChoice`](super::DialogueChoice): compares the dialogue
/// variable `var` against `value` using `op`.
///
/// An unset variable is treated as the zero value of `value`'s type (so `flag Eq Bool(false)`
/// passes when `flag` is unset). Ordered ops (`Gt`/`Lt`/`Ge`/`Le`) apply to `Int`/`Float`
/// only; on non-numeric values they evaluate to `false`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueCond {
    /// Name of the [`DialogueVars`] variable to test.
    pub var: String,
    /// Comparison operator.
    pub op: DialogueOp,
    /// Right-hand value to compare against.
    pub value: DialogueValue,
}

impl DialogueCond {
    /// A condition: `var op value`.
    pub fn new(var: impl Into<String>, op: DialogueOp, value: DialogueValue) -> Self {
        Self {
            var: var.into(),
            op,
            value,
        }
    }

    /// Evaluates the condition against `vars` (unset variable → zero value of `value`'s type).
    pub fn eval(&self, vars: &DialogueVars) -> bool {
        let actual = vars
            .get(&self.var)
            .cloned()
            .unwrap_or_else(|| self.value.zero_like());
        match self.op {
            DialogueOp::Eq => actual == self.value,
            DialogueOp::Ne => actual != self.value,
            DialogueOp::Gt | DialogueOp::Lt | DialogueOp::Ge | DialogueOp::Le => {
                match (actual.as_f64(), self.value.as_f64()) {
                    (Some(a), Some(b)) => match self.op {
                        DialogueOp::Gt => a > b,
                        DialogueOp::Lt => a < b,
                        DialogueOp::Ge => a >= b,
                        DialogueOp::Le => a <= b,
                        _ => unreachable!(),
                    },
                    _ => false, // non-numeric operand → ordered comparison is undefined
                }
            }
        }
    }
}

/// A side effect applied when a [`DialogueChoice`](super::DialogueChoice) is taken.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DialogueEffect {
    /// Set a dialogue variable in [`DialogueVars`] (grants an item, records a flag, …).
    SetVar {
        /// Variable name.
        key: String,
        /// New value.
        value: DialogueValue,
    },
    /// Emit a named [`DialogueEvent`] to `Events<DialogueEvent>` for the game to react to.
    EmitEvent {
        /// Event name the game matches on.
        name: String,
    },
}

/// Emitted to `Events<DialogueEvent>` when a chosen choice carries an
/// [`DialogueEffect::EmitEvent`]. Register the bus with
/// `App::register_event::<DialogueEvent>()` and read it in a system the same frame.
#[derive(Debug, Clone)]
pub struct DialogueEvent {
    /// The `EmitEvent` name from the chosen choice.
    pub name: String,
    /// The entity whose [`DialogueBox`](super::DialogueBox) produced the event.
    pub entity: Entity,
}

/// World resource holding named dialogue variables (flags / counters) read by choice
/// [`conditions`](DialogueCond) and written by choice [`effects`](DialogueEffect).
///
/// Game-owned: insert it (`world.insert_resource(DialogueVars::default())`) and seed starting
/// state. The [`dialogue::choose`](super::choose) helper lazily inserts it on a `SetVar` effect.
///
/// **Serializable, because an RPG's quest flags live here and a save file has to carry them.**
/// The `rpg_quest` example is what established that: it stores `has_lantern` / `knows_mine` / `gold`
/// as dialogue variables (so choice conditions can gate on them), and then had no way to persist
/// them — the getters answer one *known* key at a time, which means a save site would have to
/// hardcode the name of every flag in the game and re-`set_*` each one on load. Round-trip the whole
/// bag instead:
///
/// ```
/// # use engine::{DialogueVars, save};
/// let mut vars = DialogueVars::default();
/// vars.set_bool("has_lantern", true);
/// vars.set_int("gold", 10);
///
/// let ron = ron::to_string(&vars).unwrap();
/// let restored: DialogueVars = ron::from_str(&ron).unwrap();
/// assert_eq!(restored.get_bool("has_lantern"), Some(true));
/// assert_eq!(restored.get_int("gold"), Some(10));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DialogueVars {
    vars: HashMap<String, DialogueValue>,
}

impl DialogueVars {
    /// An empty variable store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a variable to any [`DialogueValue`].
    pub fn set(&mut self, key: impl Into<String>, value: DialogueValue) {
        self.vars.insert(key.into(), value);
    }

    /// Sets a boolean flag.
    pub fn set_bool(&mut self, key: impl Into<String>, v: bool) {
        self.set(key, DialogueValue::Bool(v));
    }

    /// Sets an integer counter.
    pub fn set_int(&mut self, key: impl Into<String>, v: i64) {
        self.set(key, DialogueValue::Int(v));
    }

    /// Sets a float.
    pub fn set_float(&mut self, key: impl Into<String>, v: f32) {
        self.set(key, DialogueValue::Float(v));
    }

    /// Sets a string.
    pub fn set_str(&mut self, key: impl Into<String>, v: impl Into<String>) {
        self.set(key, DialogueValue::Str(v.into()));
    }

    /// Raw variable lookup.
    pub fn get(&self, key: &str) -> Option<&DialogueValue> {
        self.vars.get(key)
    }

    /// Boolean lookup (`None` if unset or a different type).
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.vars.get(key).and_then(DialogueValue::as_bool)
    }

    /// Integer lookup (`None` if unset or a different type).
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.vars.get(key).and_then(DialogueValue::as_int)
    }

    /// Float lookup (`None` if unset or a different type).
    pub fn get_float(&self, key: &str) -> Option<f32> {
        self.vars.get(key).and_then(DialogueValue::as_float)
    }

    /// String lookup (`None` if unset or a different type).
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.vars.get(key).and_then(DialogueValue::as_str)
    }

    /// Every variable currently set, in unspecified order.
    ///
    /// The getters above all need a key you already know. This is for the cases that do not have
    /// one: a debug overlay listing live quest state, a test asserting on the whole bag, or a save
    /// path that wants to inspect rather than round-trip (for which `Serialize` is the shorter
    /// route). Sort by key if you need a stable order — this is a `HashMap` underneath.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &DialogueValue)> {
        self.vars.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Removes all variables.
    pub fn clear(&mut self) {
        self.vars.clear();
    }

    /// Number of stored variables.
    pub fn len(&self) -> usize {
        self.vars.len()
    }

    /// Whether no variables are set.
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save file has to carry quest state, and quest state lives here. Round-trip every value
    /// kind through RON — the format `save::write_ron` uses — and check the conditions still
    /// evaluate the same afterwards, which is what a loaded game actually depends on.
    #[test]
    fn vars_round_trip_through_ron_and_conditions_still_hold() {
        let mut vars = DialogueVars::new();
        vars.set_bool("has_lantern", true);
        vars.set_int("gold", 10);
        vars.set_float("reputation", 0.5);
        vars.set_str("last_npc", "merchant");

        let ron = ron::to_string(&vars).expect("serialize");
        let restored: DialogueVars = ron::from_str(&ron).expect("deserialize");

        assert_eq!(restored.len(), 4);
        assert_eq!(restored.get_bool("has_lantern"), Some(true));
        assert_eq!(restored.get_int("gold"), Some(10));
        assert_eq!(restored.get_float("reputation"), Some(0.5));
        assert_eq!(restored.get_str("last_npc"), Some("merchant"));
        // The gate `rpg_quest` puts on the lantern purchase must survive the round trip, not just
        // the raw value: a restored save that cannot re-evaluate its conditions is not restored.
        assert!(
            DialogueCond::new("gold", DialogueOp::Ge, DialogueValue::Int(10)).eval(&restored),
            "a gold gate that passed before saving must still pass after loading"
        );
    }

    #[test]
    fn iter_sees_every_set_var() {
        let mut vars = DialogueVars::new();
        vars.set_bool("a", true);
        vars.set_int("b", 2);
        let mut seen: Vec<&str> = vars.iter().map(|(k, _)| k).collect();
        seen.sort_unstable();
        assert_eq!(seen, vec!["a", "b"]);
        assert_eq!(vars.iter().count(), vars.len());
    }

    #[test]
    fn cond_eq_and_ne() {
        let mut vars = DialogueVars::new();
        vars.set_bool("has_key", true);
        assert!(
            DialogueCond::new("has_key", DialogueOp::Eq, DialogueValue::Bool(true)).eval(&vars)
        );
        assert!(
            DialogueCond::new("has_key", DialogueOp::Ne, DialogueValue::Bool(false)).eval(&vars)
        );
    }

    #[test]
    fn unset_var_is_zero_like() {
        let vars = DialogueVars::new();
        // unset bool compares equal to false
        assert!(DialogueCond::new("flag", DialogueOp::Eq, DialogueValue::Bool(false)).eval(&vars));
        // unset bool is NOT true
        assert!(!DialogueCond::new("flag", DialogueOp::Eq, DialogueValue::Bool(true)).eval(&vars));
        // unset int compares as 0
        assert!(DialogueCond::new("gold", DialogueOp::Lt, DialogueValue::Int(5)).eval(&vars));
    }

    #[test]
    fn ordered_numeric_comparisons() {
        let mut vars = DialogueVars::new();
        vars.set_int("gold", 7);
        assert!(DialogueCond::new("gold", DialogueOp::Ge, DialogueValue::Int(5)).eval(&vars));
        assert!(!DialogueCond::new("gold", DialogueOp::Gt, DialogueValue::Int(7)).eval(&vars));
        assert!(DialogueCond::new("gold", DialogueOp::Le, DialogueValue::Int(7)).eval(&vars));
        // int vs float cross-compare via f64
        assert!(DialogueCond::new("gold", DialogueOp::Gt, DialogueValue::Float(6.5)).eval(&vars));
    }

    #[test]
    fn ordered_op_on_non_numeric_is_false() {
        let mut vars = DialogueVars::new();
        vars.set_str("name", "abc");
        assert!(
            !DialogueCond::new("name", DialogueOp::Gt, DialogueValue::Str("a".into())).eval(&vars)
        );
    }

    #[test]
    fn vars_typed_getters() {
        let mut vars = DialogueVars::new();
        vars.set_int("n", 3);
        assert_eq!(vars.get_int("n"), Some(3));
        assert_eq!(vars.get_bool("n"), None, "wrong-type lookup is None");
        assert_eq!(vars.len(), 1);
    }
}
