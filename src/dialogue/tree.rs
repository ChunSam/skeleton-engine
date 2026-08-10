//! Data-driven dialogue trees loaded from RON.
//!
//! A [`DialogueTree`] is an *authoring* format for branching conversations: an ordered list
//! of named nodes, each with a line (literal or a localization key) and optional branching
//! [`choices`](super::DialogueChoice) whose `goto` references another node by id. At spawn it
//! flattens to an ordinary [`DialogueBox`](super::DialogueBox) — node order becomes line index
//! and every `goto` id is resolved to a line index — so the existing
//! [`DialogueSystem`](super::DialogueSystem) drives it unchanged.
//!
//! Linear flow is implicit: advancing past a node with no choices moves to the next node
//! (index `+ 1`). To jump elsewhere (skip a branch, reconverge), give the node a choice whose
//! `goto` points at the target node — the same pattern the hand-built `dialogue_branching`
//! example uses.
//!
//! # RON format
//! ```ron
//! (
//!     speaker_key: "npc.merchant",
//!     nodes: [
//!         (id: "intro", line_key: "dlg.intro"),
//!         (id: "ask",   line_key: "dlg.ask", choices: [
//!             (key: "choice.buy",  goto: "buy"),
//!             (key: "choice.dark", goto: "dark"),
//!         ]),
//!         (id: "buy",  line_key: "dlg.buy",  choices: [(key: "choice.go", goto: "bye")]),
//!         (id: "dark", line_key: "dlg.dark", choices: [(key: "choice.go", goto: "bye")]),
//!         (id: "bye",  line_key: "dlg.bye"),
//!     ],
//! )
//! ```
//! A literal (non-localized) tree uses `speaker` + per-node `line` (and choice `text`) instead
//! of the `*_key` fields. Mixing literal and localized lines in one tree is rejected.

use std::collections::HashMap;

use ron::extensions::Extensions;
use serde::Deserialize;

use super::{DialogueBox, DialogueChoice, DialogueCond, DialogueEffect};

/// Error from parsing or loading a [`DialogueTree`].
///
/// A type alias for [`crate::asset::AssetLoadError`] (shared with the animation/particle
/// loaders), keeping the public name stable while reusing the common error boilerplate.
pub type DialogueTreeError = crate::asset::AssetLoadError;

// ── Private serde def structs ────────────────────────────────────────────────

// Optional string fields use `String` with `#[serde(default)]` (empty = absent) rather than
// `Option<String>` so RON can author bare strings (`line_key: "dlg.intro"`) instead of the
// verbose `Some("dlg.intro")` that RON requires for `Option`. This matches the engine's
// existing "empty string = none" idiom (e.g. `DialogueBox::speaker`).
#[derive(Deserialize)]
struct ChoiceDef {
    #[serde(default)]
    text: String,
    #[serde(default)]
    key: String,
    goto: String,
    #[serde(default)]
    cond: Option<DialogueCond>,
    // The multi-term gates. `Vec` with `#[serde(default)]` (empty = absent) for the same reason
    // the string fields above use `String`: it authors bare in RON. All three gate fields are
    // ANDed by `DialogueChoice::is_available`, and every one of them is optional, so a file
    // written before these existed parses unchanged.
    #[serde(default)]
    cond_all: Vec<DialogueCond>,
    #[serde(default)]
    cond_any: Vec<DialogueCond>,
    #[serde(default)]
    effect: Option<DialogueEffect>,
}

#[derive(Deserialize)]
struct NodeDef {
    id: String,
    #[serde(default)]
    line: String,
    #[serde(default)]
    line_key: String,
    #[serde(default)]
    choices: Vec<ChoiceDef>,
}

fn default_cps() -> f32 {
    28.0
}

#[derive(Deserialize)]
struct DialogueTreeDef {
    #[serde(default)]
    speaker: String,
    #[serde(default)]
    speaker_key: String,
    #[serde(default = "default_cps")]
    chars_per_sec: f32,
    nodes: Vec<NodeDef>,
}

// ── DialogueTree ─────────────────────────────────────────────────────────────

/// A validated branching conversation parsed from RON, ready to flatten into a
/// [`DialogueBox`](super::DialogueBox) via [`to_box`](Self::to_box).
///
/// Parsing resolves every choice `goto` (a node id) to a line index and validates node-id
/// uniqueness, goto targets, and literal/localized consistency — so [`to_box`](Self::to_box)
/// is infallible and cheap (a clone).
#[derive(Debug, Clone)]
pub struct DialogueTree {
    /// Pre-built box template; `to_box` clones it (starts at the first node).
    template: DialogueBox,
    /// Node ids in declaration order (id of `lines[i]` is `node_ids[i]`).
    node_ids: Vec<String>,
}

impl DialogueTree {
    /// Parse a dialogue tree from RON text.
    ///
    /// Errors on: no nodes, duplicate node ids, a choice `goto` that references an unknown
    /// node id, or a tree that mixes localized (`line_key`) and literal (`line`) nodes.
    pub fn from_ron_str(s: &str) -> Result<Self, DialogueTreeError> {
        // IMPLICIT_SOME lets optional struct fields (a choice's `cond`/`effect`) be authored
        // inline (e.g. `cond: (var: "gold", op: Ge, value: Int(5))`) instead of RON's verbose
        // `Some(...)` wrapper.
        let options = ron::Options::default().with_default_extension(Extensions::IMPLICIT_SOME);
        let def: DialogueTreeDef = options
            .from_str(s)
            .map_err(|e| DialogueTreeError::Ron(e.to_string()))?;

        if def.nodes.is_empty() {
            return Err(DialogueTreeError::Ron("dialogue tree has no nodes".into()));
        }

        // Node id → line index (declaration order); reject duplicates.
        let mut id_to_index: HashMap<String, usize> = HashMap::with_capacity(def.nodes.len());
        for (i, n) in def.nodes.iter().enumerate() {
            if id_to_index.insert(n.id.clone(), i).is_some() {
                return Err(DialogueTreeError::Ron(format!(
                    "duplicate node id '{}'",
                    n.id
                )));
            }
        }

        // A tree is localized if any node carries a line_key; require all-or-nothing.
        let localized = def.nodes.iter().any(|n| !n.line_key.is_empty());
        if localized {
            if let Some(n) = def.nodes.iter().find(|n| n.line_key.is_empty()) {
                return Err(DialogueTreeError::Ron(format!(
                    "node '{}' has no line_key but the tree is localized \
                     (mixing literal and localized lines is not allowed)",
                    n.id
                )));
            }
        }

        // Flatten nodes → DialogueBox (node order = line index).
        let mut dbox = if localized {
            let line_keys: Vec<String> = def.nodes.iter().map(|n| n.line_key.clone()).collect();
            DialogueBox::localized(def.speaker_key.clone(), line_keys)
        } else {
            let lines: Vec<String> = def.nodes.iter().map(|n| n.line.clone()).collect();
            DialogueBox::new(def.speaker.clone(), lines)
        }
        .with_chars_per_sec(def.chars_per_sec);

        // Attach branching choices, resolving each goto id → line index.
        for (i, n) in def.nodes.iter().enumerate() {
            if n.choices.is_empty() {
                continue;
            }
            let mut choices = Vec::with_capacity(n.choices.len());
            for c in &n.choices {
                let goto = *id_to_index.get(&c.goto).ok_or_else(|| {
                    DialogueTreeError::Ron(format!(
                        "node '{}' choice goto references unknown node id '{}'",
                        n.id, c.goto
                    ))
                })?;
                let mut choice = if c.key.is_empty() {
                    DialogueChoice::new(c.text.clone(), goto)
                } else {
                    DialogueChoice::localized(c.key.clone(), goto)
                };
                if let Some(cond) = &c.cond {
                    choice = choice.when(cond.clone());
                }
                if !c.cond_all.is_empty() {
                    choice = choice.when_all(c.cond_all.iter().cloned());
                }
                if !c.cond_any.is_empty() {
                    choice = choice.when_any(c.cond_any.iter().cloned());
                }
                if let Some(effect) = &c.effect {
                    choice = choice.then(effect.clone());
                }
                choices.push(choice);
            }
            dbox = dbox.with_choices(i, choices);
        }

        let node_ids = def.nodes.iter().map(|n| n.id.clone()).collect();
        Ok(DialogueTree {
            template: dbox,
            node_ids,
        })
    }

    /// Load a dialogue tree from a file path (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(path: &str) -> Result<Self, DialogueTreeError> {
        let text = std::fs::read_to_string(crate::asset_path::resolve(path))?;
        Self::from_ron_str(&text)
    }

    /// Build a fresh [`DialogueBox`](super::DialogueBox) from this tree (starts at the first
    /// node). Cheap — clones the validated template; add it to an entity to start the
    /// conversation.
    pub fn to_box(&self) -> DialogueBox {
        self.template.clone()
    }

    /// Node ids in declaration order; the id of line `i` is `node_ids()[i]`.
    pub fn node_ids(&self) -> &[String] {
        &self.node_ids
    }
}

// ── DialogueRegistry ─────────────────────────────────────────────────────────

/// Registry of named [`DialogueTree`]s, stored as a World resource.
///
/// Load trees via [`App::load_dialogue`](crate::App::load_dialogue); build a box from one at
/// runtime with [`box_of`](Self::box_of) (or [`get`](Self::get) + [`DialogueTree::to_box`]).
#[derive(Default)]
pub struct DialogueRegistry {
    inner: crate::ron_registry::RonRegistry<DialogueTree>,
}

impl DialogueRegistry {
    /// Load a dialogue tree from `path` and register it under `name` (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, name: impl Into<String>, path: &str) -> Result<(), DialogueTreeError> {
        self.inner.load(name, path)
    }

    /// Insert a tree directly (useful for tests or in-memory trees).
    pub fn insert(&mut self, name: impl Into<String>, tree: DialogueTree) {
        self.inner.insert(name, tree);
    }

    /// Look up a registered tree by name.
    pub fn get(&self, name: &str) -> Option<&DialogueTree> {
        self.inner.get(name)
    }

    /// Build a fresh [`DialogueBox`](super::DialogueBox) from the named tree, or `None` if no
    /// such tree is registered.
    pub fn box_of(&self, name: &str) -> Option<DialogueBox> {
        self.inner.get(name).map(DialogueTree::to_box)
    }

    /// Re-load the tree whose source path matches `path` from disk.
    ///
    /// No-op if no tree with that path is registered or if file read/parse fails (logs a
    /// warning in that case).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload_path(&mut self, path: &str) {
        self.inner.reload_path(path, "dialogue");
    }

    /// Sorted list of registered tree names.
    pub fn names(&self) -> Vec<String> {
        self.inner.names()
    }
}

// ── RonLoadable + HotReloadable impls ─────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
impl crate::ron_registry::RonLoadable for DialogueTree {
    type Err = DialogueTreeError;
    fn load_ron(path: &str) -> Result<Self, Self::Err> {
        DialogueTree::load(path)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::asset::HotReloadable for DialogueRegistry {
    fn reload_path(&mut self, path: &str) {
        // Delegate to the inherent method via UFCS to avoid same-name ambiguity / recursion.
        DialogueRegistry::reload_path(self, path);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const LITERAL_RON: &str = r#"(
    speaker: "Merchant",
    nodes: [
        (id: "intro", line: "Welcome."),
        (id: "ask",   line: "Buy or leave?", choices: [
            (text: "Buy",   goto: "buy"),
            (text: "Leave", goto: "bye"),
        ]),
        (id: "buy",  line: "A fine choice.", choices: [(text: "Go", goto: "bye")]),
        (id: "bye",  line: "Farewell."),
    ],
)"#;

    const LOCALIZED_RON: &str = r#"(
    speaker_key: "npc.merchant",
    chars_per_sec: 40.0,
    nodes: [
        (id: "intro", line_key: "dlg.intro"),
        (id: "ask",   line_key: "dlg.ask", choices: [
            (key: "choice.buy",  goto: "buy"),
            (key: "choice.dark", goto: "bye"),
        ]),
        (id: "buy",  line_key: "dlg.buy", choices: [(key: "choice.go", goto: "bye")]),
        (id: "bye",  line_key: "dlg.bye"),
    ],
)"#;

    #[test]
    fn literal_tree_flattens_to_box_with_index_gotos() {
        let tree = DialogueTree::from_ron_str(LITERAL_RON).expect("parse");
        let b = tree.to_box();
        assert_eq!(b.speaker, "Merchant");
        assert_eq!(
            b.lines,
            vec!["Welcome.", "Buy or leave?", "A fine choice.", "Farewell."]
        );
        assert_eq!(tree.node_ids(), ["intro", "ask", "buy", "bye"]);
        // "ask" is line 1: Buy → index 2 ("buy"), Leave → index 3 ("bye").
        let ask = b
            .choices
            .iter()
            .find(|(l, _)| *l == 1)
            .expect("ask choices");
        assert_eq!(ask.1[0].goto, 2);
        assert_eq!(ask.1[0].text, "Buy");
        assert_eq!(ask.1[1].goto, 3);
        // "buy" is line 2: Go → index 3 ("bye").
        let buy = b
            .choices
            .iter()
            .find(|(l, _)| *l == 2)
            .expect("buy choices");
        assert_eq!(buy.1[0].goto, 3);
    }

    #[test]
    fn localized_tree_builds_localized_box() {
        let tree = DialogueTree::from_ron_str(LOCALIZED_RON).expect("parse");
        let b = tree.to_box();
        assert!(
            b.lines.is_empty(),
            "localized box starts with empty lines (resolved at runtime)"
        );
        assert_eq!(
            b.line_keys,
            vec!["dlg.intro", "dlg.ask", "dlg.buy", "dlg.bye"]
        );
        assert_eq!(b.speaker_key.as_deref(), Some("npc.merchant"));
        assert!((b.chars_per_sec - 40.0).abs() < 1e-6);
        // localized choices carry keys, goto resolved to index.
        let ask = b
            .choices
            .iter()
            .find(|(l, _)| *l == 1)
            .expect("ask choices");
        assert_eq!(ask.1[0].key.as_deref(), Some("choice.buy"));
        assert_eq!(ask.1[0].goto, 2);
        assert_eq!(ask.1[1].goto, 3);
    }

    #[test]
    fn to_box_is_independent_clone() {
        let tree = DialogueTree::from_ron_str(LITERAL_RON).expect("parse");
        let mut a = tree.to_box();
        a.advance();
        let b = tree.to_box();
        assert_eq!(b.current, 0, "a second box starts fresh");
    }

    #[test]
    fn duplicate_node_id_is_err() {
        let ron = r#"(nodes: [(id: "x", line: "a"), (id: "x", line: "b")])"#;
        let err = DialogueTree::from_ron_str(ron).unwrap_err();
        assert!(format!("{err}").contains("duplicate node id"), "got: {err}");
    }

    #[test]
    fn unknown_goto_is_err() {
        let ron = r#"(nodes: [(id: "a", line: "x", choices: [(text: "go", goto: "nope")])])"#;
        let err = DialogueTree::from_ron_str(ron).unwrap_err();
        assert!(
            format!("{err}").contains("unknown node id 'nope'"),
            "got: {err}"
        );
    }

    #[test]
    fn mixed_literal_and_localized_is_err() {
        let ron = r#"(nodes: [(id: "a", line_key: "k"), (id: "b", line: "literal")])"#;
        let err = DialogueTree::from_ron_str(ron).unwrap_err();
        assert!(format!("{err}").contains("localized"), "got: {err}");
    }

    #[test]
    fn empty_tree_is_err() {
        let err = DialogueTree::from_ron_str(r#"(nodes: [])"#).unwrap_err();
        assert!(format!("{err}").contains("no nodes"), "got: {err}");
    }

    #[test]
    fn tree_parses_cond_and_effect() {
        let ron = r#"(
    nodes: [
        (id: "ask", line: "Pick", choices: [
            (text: "vip", goto: "end",
             cond: (var: "vip", op: Eq, value: Bool(true)),
             effect: SetVar(key: "chose_vip", value: Bool(true))),
        ]),
        (id: "end", line: "Bye"),
    ],
)"#;
        let tree = DialogueTree::from_ron_str(ron).expect("parse");
        let b = tree.to_box();
        let ask = b.choices.iter().find(|(l, _)| *l == 0).expect("ask");
        let c = &ask.1[0];
        assert_eq!(c.goto, 1, "goto id 'end' → index 1");
        assert!(c.cond.is_some(), "cond parsed via IMPLICIT_SOME");
        assert!(c.effect.is_some(), "effect parsed via IMPLICIT_SOME");
        // The single-term form leaves the multi-term gates empty, so it stays unconditional-ish:
        // exactly one gate is in play, which is what every pre-`cond_all` file expects.
        assert!(c.cond_all.is_empty() && c.cond_any.is_empty());
    }

    /// **The additivity claim, asserted rather than assumed.** `cond_all` / `cond_any` were added
    /// after `.dlg.ron` files were already being authored, and the whole reason they are flat
    /// fields on the choice — instead of `All`/`Any` variants on `DialogueCond` — is that RON 0.8
    /// cannot add those variants without rewriting every existing `cond:` line. This is the text
    /// of a tree written before the feature existed; it must still parse to the same box.
    #[test]
    fn a_pre_feature_tree_parses_unchanged() {
        const PRE_FEATURE: &str = r#"(
    speaker_key: "npc.merchant",
    chars_per_sec: 40.0,
    nodes: [
        (id: "ask", line_key: "dlg.ask", choices: [
            (key: "choice.buy", goto: "buy",
             cond: (var: "can_buy_lantern", op: Eq, value: Bool(true)),
             effect: EmitEvent(name: "buy_lantern")),
            (key: "choice.leave", goto: "buy"),
        ]),
        (id: "buy", line_key: "dlg.buy"),
    ],
)"#;
        let b = DialogueTree::from_ron_str(PRE_FEATURE)
            .expect("a tree written before cond_all/cond_any existed must still parse")
            .to_box();
        let choices = &b.choices.iter().find(|(l, _)| *l == 0).expect("ask").1;
        assert_eq!(choices.len(), 2);
        assert_eq!(
            choices[0].cond.as_ref().expect("cond").var,
            "can_buy_lantern"
        );
        assert!(
            choices[0].cond_all.is_empty(),
            "absent field defaults empty"
        );
        assert!(
            choices[0].cond_any.is_empty(),
            "absent field defaults empty"
        );
        assert!(choices[1].is_unconditional(), "ungated choice unaffected");
    }

    /// The new gates author in RON, and both directions of each are driven — a tree that parsed
    /// the fields but never evaluated them would pass a visible-only assertion.
    #[test]
    fn tree_parses_multi_term_gates() {
        let ron = r#"(
    chars_per_sec: 0.0,
    nodes: [
        (id: "ask", line: "Pick", choices: [
            (text: "buy", goto: "end", cond_all: [
                (var: "gold", op: Ge, value: Int(10)),
                (var: "has_lantern", op: Eq, value: Bool(false)),
            ]),
            (text: "nodeal", goto: "end", cond_any: [
                (var: "gold", op: Lt, value: Int(10)),
                (var: "has_lantern", op: Eq, value: Bool(true)),
            ]),
        ]),
        (id: "end", line: "Bye"),
    ],
)"#;
        let b = DialogueTree::from_ron_str(ron).expect("parse").to_box();
        let choices = &b.choices.iter().find(|(l, _)| *l == 0).expect("ask").1;
        assert_eq!(choices[0].cond_all.len(), 2, "both terms survived parsing");
        assert_eq!(choices[1].cond_any.len(), 2);
        assert!(choices[0].cond.is_none(), "cond_all does not populate cond");

        // The parsed gates are mutually exclusive by construction — whichever vars are set,
        // exactly one of the two branches is offered. That is the property the tree encodes.
        let d = b.clone();
        for (gold, lantern, want) in [
            (10, false, "buy"),
            (9, false, "nodeal"),
            (10, true, "nodeal"),
            (9, true, "nodeal"),
        ] {
            let mut vars = crate::DialogueVars::new();
            vars.set_int("gold", gold);
            vars.set_bool("has_lantern", lantern);
            let visible = d.visible_choices(&vars);
            assert_eq!(
                visible.len(),
                1,
                "gold={gold} lantern={lantern}: exactly one branch should be offered"
            );
            assert_eq!(visible[0].text, want, "gold={gold} lantern={lantern}");
        }
    }

    #[test]
    fn registry_insert_get_and_box_of() {
        let tree = DialogueTree::from_ron_str(LITERAL_RON).expect("parse");
        let mut reg = DialogueRegistry::default();
        reg.insert("merchant", tree);
        assert!(reg.get("merchant").is_some());
        assert!(reg.get("missing").is_none());
        let b = reg.box_of("merchant").expect("box");
        assert_eq!(b.lines.len(), 4);
        assert!(reg.box_of("missing").is_none());
        assert_eq!(reg.names(), vec!["merchant"]);
    }
}
