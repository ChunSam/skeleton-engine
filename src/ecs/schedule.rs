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
    /// Circular dependency. Contains the indices of the systems in the cycle.
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
    use std::collections::HashMap;

    let n = metas.len();

    // label → indices that carry that label
    let mut by_label: HashMap<SystemLabel, Vec<usize>> = HashMap::new();
    for (i, m) in metas.iter().enumerate() {
        if let Some(l) = m.label {
            by_label.entry(l).or_default().push(i);
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

    // In-degree
    let mut indeg = vec![0usize; n];
    for &(_, to) in &edges {
        indeg[to] += 1;
    }

    // Kahn's algorithm — deterministic: always pick the lowest index among those with in-degree 0
    let mut order = Vec::with_capacity(n);
    let mut available: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();

    while let Some(&next) = available.iter().min() {
        available.retain(|&x| x != next);
        order.push(next);
        for &(from, to) in &edges {
            if from == next {
                indeg[to] -= 1;
                if indeg[to] == 0 {
                    available.push(to);
                }
            }
        }
    }

    if order.len() != n {
        let remaining: Vec<usize> = (0..n).filter(|i| !order.contains(i)).collect();
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
}
