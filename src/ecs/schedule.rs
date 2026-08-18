/// System label type. Used to identify ordering and groups.
pub type SystemLabel = &'static str;

/// Ordering/group configuration passed to `add_system_labeled`. Builder pattern.
#[derive(Default, Clone)]
pub struct SystemConfig {
    pub(crate) label: Option<SystemLabel>,
    pub(crate) before: Vec<SystemLabel>,
    pub(crate) after: Vec<SystemLabel>,
    pub(crate) set: Option<SystemLabel>,
}

impl SystemConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches a label to this system. Other systems can reference it via before/after.
    pub fn label(mut self, l: SystemLabel) -> Self {
        self.label = Some(l);
        self
    }

    /// Requests that this system run **before** the system with the given label.
    pub fn before(mut self, l: SystemLabel) -> Self {
        self.before.push(l);
        self
    }

    /// Requests that this system run **after** the system with the given label.
    pub fn after(mut self, l: SystemLabel) -> Self {
        self.after.push(l);
        self
    }

    /// Places this system in the given SystemSet. If that set is disabled, the system is skipped.
    pub fn in_set(mut self, s: SystemLabel) -> Self {
        self.set = Some(s);
        self
    }
}

/// Schedule computation error.
#[derive(Debug, PartialEq)]
pub enum ScheduleError {
    /// Circular dependency. Contains every system that never became runnable — the cycle
    /// **plus everything downstream of it**, because a system waiting on a cycle never reaches
    /// in-degree 0 either.
    ///
    /// Do not read it as "these systems form the cycle": in a 40-system schedule a genuine
    /// two-system cycle can name thirty innocent bystanders, and the first thirty registrations
    /// you inspect will be fine. Narrow it by looking for the mutual `before`/`after` pair
    /// *among* these indices.
    Cycle(Vec<usize>),
}

/// Computes the execution order via topological sort.
///
/// - Input: metadata for each system (index order = insertion order)
/// - Edges: `after(X)` → all systems with label X run before self.
///   `before(Y)` → self runs before systems with label Y.
/// - Tie-breaker for equal rank is insertion order (ascending index), making the result deterministic.
/// - Success: `Ok(execution index order)`. Cycle: `Err(Cycle(remaining indices))`.
pub fn compute_order(metas: &[SystemConfig]) -> Result<Vec<usize>, ScheduleError> {
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashMap};

    let n = metas.len();

    // label → indices that carry that label
    let mut by_label: HashMap<SystemLabel, Vec<usize>> = HashMap::new();
    for (i, m) in metas.iter().enumerate() {
        if let Some(l) = m.label {
            by_label.entry(l).or_default().push(i);
        }
    }

    // A dangling `after`/`before` reference is always a bug, never intent — but the loops below
    // simply skip it, so the ordering the author wrote silently does not exist and the schedule
    // falls back to insertion order. That is invisible until the day insertion order stops
    // agreeing with the intent, and then it presents as a frame-ordering glitch nowhere near
    // the registration.
    //
    // The dominant cause is NOT a typo. `add`/`add_system` attach `SystemConfig::default()`,
    // whose `label` is `None`, so `X::LABEL` names nothing at all unless X was itself registered
    // with `add_system_labeled(x, SystemConfig::new().label(X::LABEL))` — a `LABEL` constant is
    // just a `&'static str`, not a self-registering identity.
    for (i, m) in metas.iter().enumerate() {
        for l in m.after.iter().chain(m.before.iter()) {
            if !by_label.contains_key(l) {
                log::warn!(
                    "system #{i} is ordered against label {l:?}, but no registered system carries \
                     that label — the constraint is being IGNORED and ordering falls back to \
                     insertion order. Register the target with \
                     `add_system_labeled(sys, SystemConfig::new().label({l:?}))`."
                );
            } else if m.label == Some(*l) {
                // The other half of the same silent-ordering class, and the one the dangling
                // check above waves through: the label DOES exist, so nothing looked wrong,
                // but the only edge it could produce points from the system to itself and is
                // dropped by the `s != i` guard below. `.label(X).after(X)` — the typo shape of
                // "I meant `.after(Y)`" — therefore yields zero constraints and zero warnings.
                log::warn!(
                    "system #{i} carries label {l:?} and is ALSO ordered against it. A system \
                     cannot run before or after itself, so that self-edge is dropped. If this \
                     is a typo for another label, fix it; if you meant a barrier against the \
                     OTHER systems sharing {l:?}, note the constraint holds only against them, \
                     never against this one."
                );
            }
        }
    }

    // Edge set (from → to). HashSet to prevent duplicates.
    let mut edges: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();

    for (i, m) in metas.iter().enumerate() {
        // after(a): systems with label a come before i
        for a in &m.after {
            if let Some(srcs) = by_label.get(a) {
                for &s in srcs {
                    if s != i {
                        edges.insert((s, i));
                    }
                }
            }
        }
        // before(b): i comes before systems with label b
        for b in &m.before {
            if let Some(dsts) = by_label.get(b) {
                for &d in dsts {
                    if d != i {
                        edges.insert((i, d));
                    }
                }
            }
        }
    }

    // Adjacency list + in-degrees, built once from the edge set.
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut indeg = vec![0usize; n];
    for &(from, to) in &edges {
        adj[from].push(to);
        indeg[to] += 1;
    }

    // Kahn's algorithm — deterministic: always pick the lowest index among those with
    // in-degree 0. A min-heap (`Reverse`) keeps that tie-break at O(log n) per pop, and the
    // adjacency list relaxes each node's out-edges exactly once — O((V + E) log V) overall,
    // versus the previous full-edge rescan per pop (O(V·E)) plus the O(V) `min()`/`retain`.
    let mut ready: BinaryHeap<Reverse<usize>> =
        (0..n).filter(|&i| indeg[i] == 0).map(Reverse).collect();
    let mut order = Vec::with_capacity(n);

    while let Some(Reverse(next)) = ready.pop() {
        order.push(next);
        for &to in &adj[next] {
            indeg[to] -= 1;
            if indeg[to] == 0 {
                ready.push(Reverse(to));
            }
        }
    }

    if order.len() != n {
        // A node never reaching in-degree 0 is in (or downstream of) a cycle. Equivalent to the
        // old `!order.contains(i)` test, since popped ⟺ in-degree hit 0.
        let remaining: Vec<usize> = (0..n).filter(|&i| indeg[i] > 0).collect();
        return Err(ScheduleError::Cycle(remaining));
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta_default() -> SystemConfig {
        SystemConfig::default()
    }

    fn meta_label(label: &'static str) -> SystemConfig {
        SystemConfig {
            label: Some(label),
            ..Default::default()
        }
    }

    fn meta_label_after(label: &'static str, after: &'static str) -> SystemConfig {
        SystemConfig {
            label: Some(label),
            after: vec![after],
            ..Default::default()
        }
    }

    fn meta_label_before(label: &'static str, before: &'static str) -> SystemConfig {
        SystemConfig {
            label: Some(label),
            before: vec![before],
            ..Default::default()
        }
    }

    fn meta_after(after: &'static str) -> SystemConfig {
        SystemConfig {
            after: vec![after],
            ..Default::default()
        }
    }

    /// 1. Three unconstrained systems → insertion order preserved
    #[test]
    fn no_constraints_keeps_insertion_order() {
        let metas = vec![meta_default(), meta_default(), meta_default()];
        let order = compute_order(&metas).unwrap();
        assert_eq!(order, vec![0, 1, 2]);
    }

    /// 2. after constraint — sys1(label "b", after "a"), sys0(label "a") → 0 comes before 1
    #[test]
    fn after_orders_correctly() {
        // index 0: label "a"
        // index 1: label "b", after "a"
        let metas = vec![meta_label("a"), meta_label_after("b", "a")];
        let order = compute_order(&metas).unwrap();
        let pos0 = order.iter().position(|&x| x == 0).unwrap();
        let pos1 = order.iter().position(|&x| x == 1).unwrap();
        assert!(
            pos0 < pos1,
            "label 'a'(idx 0) should come before label 'b'(idx 1)"
        );
    }

    /// 3. before constraint — sys0(label "a", before "b"), sys1(label "b") → 0 comes before 1
    #[test]
    fn before_orders_correctly() {
        // index 0: label "a", before "b"
        // index 1: label "b"
        let metas = vec![meta_label_before("a", "b"), meta_label("b")];
        let order = compute_order(&metas).unwrap();
        let pos0 = order.iter().position(|&x| x == 0).unwrap();
        let pos1 = order.iter().position(|&x| x == 1).unwrap();
        assert!(
            pos0 < pos1,
            "label 'a'(idx 0) should come before label 'b'(idx 1)"
        );
    }

    /// 4. Cycle detection
    #[test]
    fn cycle_detected() {
        // index 0: label "a", after "b"
        // index 1: label "b", after "a"
        let metas = vec![meta_label_after("a", "b"), meta_label_after("b", "a")];
        let result = compute_order(&metas);
        assert!(
            matches!(result, Err(ScheduleError::Cycle(_))),
            "a circular dependency should return Err(Cycle(..))"
        );
    }

    /// 5. Shared-label barrier — two systems with label "render", another after "render"
    #[test]
    fn shared_label_barrier() {
        // index 0: label "render"
        // index 1: label "render"
        // index 2: after "render" (both render systems must come before 2)
        let metas = vec![
            meta_label("render"),
            meta_label("render"),
            meta_after("render"),
        ];
        let order = compute_order(&metas).unwrap();
        let pos0 = order.iter().position(|&x| x == 0).unwrap();
        let pos1 = order.iter().position(|&x| x == 1).unwrap();
        let pos2 = order.iter().position(|&x| x == 2).unwrap();
        assert!(
            pos0 < pos2,
            "render(idx 0) should come before after_render(idx 2)"
        );
        assert!(
            pos1 < pos2,
            "render(idx 1) should come before after_render(idx 2)"
        );
    }

    /// An `after` naming a label **no registered system carries** creates no constraint at all.
    ///
    /// This is the shape of the trap `compute_order` now warns about. `add`/`add_system` attach
    /// `SystemConfig::default()`, whose `label` is `None` — a `LABEL` constant is just a
    /// `&'static str`, not a self-registering identity. So the extremely natural
    /// `systems.add(LayoutSystem); systems.add_labeled(UiSystem, SystemConfig::new()
    /// .after(LayoutSystem::LABEL))` produced **zero** edges, and the ordering held only by the
    /// accident of insertion order. The engine's own rustdoc taught that form, and four examples
    /// copied it.
    ///
    /// Pinned as an assertion on the *edge*, not on insertion order: reversing the registration
    /// order is what tells the two cases apart.
    /// A system ordered against **its own** label gets no constraint at all — the label exists,
    /// so the dangling check waves it through, and the only edge it could make points from the
    /// system to itself and is dropped. `.label(X).after(X)` is the typo shape of
    /// "I meant `.after(Y)`", and it used to produce zero edges and zero diagnostics.
    ///
    /// Pinned on the edge, like the dangling case below: reversing registration order is what
    /// tells "no constraint" apart from "constraint that happens to agree with insertion order".
    #[test]
    fn self_referencing_label_creates_no_constraint() {
        // index 0 carries "layout" AND asks to run after "layout" — itself.
        let metas = vec![meta_label_after("layout", "layout"), meta_label("other")];
        let order = compute_order(&metas).unwrap();
        assert_eq!(
            order,
            vec![0, 1],
            "a self-edge must be dropped, leaving pure insertion order; got {order:?}"
        );

        // Same shape with the registration order flipped: still insertion order, i.e. still
        // genuinely unconstrained rather than accidentally agreeing.
        let metas = vec![meta_label("other"), meta_label_after("layout", "layout")];
        let order = compute_order(&metas).unwrap();
        assert_eq!(order, vec![0, 1], "got {order:?}");
    }

    /// A shared label is a barrier against the OTHER holders only. Two systems labeled
    /// "render", the second also `.after("render")`: the edge from the first is real, the
    /// self-edge is not, so the author's "after both" reading is half-true.
    #[test]
    fn shared_label_after_self_orders_against_the_other_holder_only() {
        let metas = vec![meta_label("render"), meta_label_after("render", "render")];
        let order = compute_order(&metas).unwrap();
        let pos0 = order.iter().position(|&x| x == 0).unwrap();
        let pos1 = order.iter().position(|&x| x == 1).unwrap();
        assert!(
            pos0 < pos1,
            "the OTHER holder still orders it; got {order:?}"
        );
    }

    /// `Cycle` reports every system that never became runnable, which is the cycle plus its
    /// downstream — not the cycle alone. Pins the documented contract.
    #[test]
    fn cycle_reports_downstream_systems_too() {
        // 0 <-> 1 is the cycle; 2 merely waits on it and is blocked by association.
        let metas = vec![
            meta_label_after("a", "b"),
            meta_label_after("b", "a"),
            meta_label_after("c", "a"),
        ];
        match compute_order(&metas) {
            Err(ScheduleError::Cycle(blocked)) => assert_eq!(
                blocked,
                vec![0, 1, 2],
                "index 2 is downstream, not part of the cycle, and is reported anyway"
            ),
            other => panic!("expected a cycle, got {other:?}"),
        }
    }

    #[test]
    fn dangling_after_label_creates_no_constraint() {
        // index 0 wants to run after "layout", but index 1 carries NO label.
        let metas = vec![meta_after("layout"), SystemConfig::default()];
        let order = compute_order(&metas).unwrap();
        let pos0 = order.iter().position(|&x| x == 0).unwrap();
        let pos1 = order.iter().position(|&x| x == 1).unwrap();
        assert!(
            pos0 < pos1,
            "a dangling label must add no edge, leaving pure insertion order; got {order:?}"
        );

        // Label the target and the very same `after` now genuinely orders them.
        let metas = vec![meta_after("layout"), meta_label("layout")];
        let order = compute_order(&metas).unwrap();
        let pos0 = order.iter().position(|&x| x == 0).unwrap();
        let pos1 = order.iter().position(|&x| x == 1).unwrap();
        assert!(
            pos1 < pos0,
            "with the target labeled, `after` must reorder against insertion order; got {order:?}"
        );
    }
}
