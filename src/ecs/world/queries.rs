//! Read-only and mutable iteration over components (`query`, `query2`..`query4`,
//! the `*_mut` variants, and the `_with` / `_without` / `_opt2` filters).

use super::{Archetype, Entity, World};
use std::any::TypeId;

impl World {
    /// Iterates over all (Entity, &T) pairs that have component T.
    pub fn query<T: 'static>(&self) -> impl Iterator<Item = (Entity, &T)> {
        let tid = TypeId::of::<T>();
        self.archetypes
            .iter()
            .filter(move |arch| arch.contains(tid))
            .flat_map(move |arch| {
                arch.debug_assert_columns_aligned();
                let col = arch
                    .columns
                    .get(&tid)
                    .expect("query: archetype was filtered to contain this column");
                arch.entities
                    .iter()
                    .zip(col.iter())
                    .map(|(&e, c)| (e, c.downcast_ref::<T>().expect("column holds type T")))
            })
    }

    /// Iterates over all `(Entity, &mut T)` pairs with **mutable** references.
    ///
    /// Mutable variant of `query::<T>()`. Avoids the two-pass workaround
    /// ("collect entities then `get_mut`", which allocates every frame) when you
    /// want to mutate a single component across many entities.
    ///
    /// As with `get_mut`, changes made here are NOT automatically recorded in
    /// `query_changed<T>()`. Call [`World::mark_changed`] if change detection is needed.
    pub fn query_mut<T: 'static>(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        let tid = TypeId::of::<T>();
        self.archetypes
            .iter_mut()
            .filter(move |arch| arch.contains(tid))
            .flat_map(move |arch| {
                arch.debug_assert_columns_aligned();
                // `entities` (immutable) and `columns` (mutable) are distinct fields of
                // Archetype, so destructuring lets us borrow both disjointly at once.
                let Archetype {
                    entities, columns, ..
                } = arch;
                let col = columns
                    .get_mut(&tid)
                    .expect("query_mut: archetype was filtered to contain this column");
                entities
                    .iter()
                    .zip(col.iter_mut())
                    .map(|(&e, c)| (e, c.downcast_mut::<T>().expect("column holds type T")))
            })
    }

    /// Mutable two-component query: `(Entity, &mut A, &mut B)` for entities that have both.
    ///
    /// The mutable analogue of [`query2`](World::query2). Avoids the "collect the entities, then
    /// `get_mut` each twice" workaround (which allocates a `Vec` every frame) when a system updates
    /// two components together. **`A` and `B` must be distinct types** (two `&mut` to the same
    /// component would alias).
    ///
    /// As with [`get_mut`](World::get_mut), changes are NOT recorded in
    /// [`query_changed`](World::query_changed); call [`mark_changed`](World::mark_changed) if change
    /// detection is needed.
    pub fn query2_mut<A: 'static, B: 'static>(
        &mut self,
    ) -> impl Iterator<Item = (Entity, &mut A, &mut B)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        // Eagerly, before the iterator is built, because the alternative fails only when the
        // data happens to reach it: `get_disjoint_mut` panics on overlapping keys, but only
        // once a matching archetype exists, so `query2_mut::<A, A>()` returned an empty
        // iterator on an empty World and killed the system the moment one entity showed up.
        // A test could pass and the game still die. `assert` rather than `debug_assert`: two
        // `&mut` to one component is always a bug, the check is one TypeId compare per query
        // call (not per entity), and std's own message names neither the query nor the type.
        assert_ne!(
            ta,
            tb,
            "query2_mut::<A, B>() requires two DISTINCT component types — both were {}. \
             Two `&mut` to the same component would alias; use query_mut::<T>() for one type.",
            std::any::type_name::<A>()
        );
        self.archetypes
            .iter_mut()
            .filter(move |arch| arch.contains(ta) && arch.contains(tb))
            .flat_map(move |arch| {
                arch.debug_assert_columns_aligned();
                // `entities` (read) and `columns` (write) are distinct fields → disjoint borrows.
                let Archetype {
                    entities, columns, ..
                } = arch;
                // Two DISTINCT columns need simultaneous `&mut`; `get_mut` twice would alias the
                // map, so `get_disjoint_mut` hands back both at once.
                let [ca, cb] = columns.get_disjoint_mut([&ta, &tb]);
                let ca = ca.expect("query2_mut: column A present (archetype was filtered for it)");
                let cb = cb.expect("query2_mut: column B present (archetype was filtered for it)");
                entities
                    .iter()
                    .zip(ca.iter_mut())
                    .zip(cb.iter_mut())
                    .map(|((&e, a), b)| {
                        (
                            e,
                            a.downcast_mut::<A>().expect("column holds type A"),
                            b.downcast_mut::<B>().expect("column holds type B"),
                        )
                    })
            })
    }

    /// Mutable three-component query: `(Entity, &mut A, &mut B, &mut C)`. See [`query2_mut`](World::query2_mut);
    /// `A`, `B`, `C` must be distinct types.
    pub fn query3_mut<A: 'static, B: 'static, C: 'static>(
        &mut self,
    ) -> impl Iterator<Item = (Entity, &mut A, &mut B, &mut C)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        let tc = TypeId::of::<C>();
        // Eagerly and pairwise — see `query2_mut` for why this is not a `debug_assert`.
        //
        // The message names the colliding PAIR, not all three types. Printing three names and
        // leaving the reader to spot the duplicate is the eyeballing `query2_mut`'s message was
        // written to remove, and it is worse here: with three long paths (`game::comp::Position`
        // and friends) the repeat is genuinely easy to miss. The check is pairwise already, so the
        // pair is known at the point of failure — say it.
        if let Some((x, y, name)) = if ta == tb {
            Some(("A", "B", std::any::type_name::<A>()))
        } else if ta == tc {
            Some(("A", "C", std::any::type_name::<A>()))
        } else if tb == tc {
            Some(("B", "C", std::any::type_name::<B>()))
        } else {
            None
        } {
            panic!(
                "query3_mut::<A, B, C>() requires three DISTINCT component types, but {x} and {y} \
                 are both `{name}`. Two `&mut` to the same component would alias."
            );
        }
        self.archetypes
            .iter_mut()
            .filter(move |arch| arch.contains(ta) && arch.contains(tb) && arch.contains(tc))
            .flat_map(move |arch| {
                arch.debug_assert_columns_aligned();
                let Archetype {
                    entities, columns, ..
                } = arch;
                let [ca, cb, cc] = columns.get_disjoint_mut([&ta, &tb, &tc]);
                let ca = ca.expect("query3_mut: column A present");
                let cb = cb.expect("query3_mut: column B present");
                let cc = cc.expect("query3_mut: column C present");
                entities
                    .iter()
                    .zip(ca.iter_mut())
                    .zip(cb.iter_mut())
                    .zip(cc.iter_mut())
                    .map(|(((&e, a), b), c)| {
                        (
                            e,
                            a.downcast_mut::<A>().expect("column holds type A"),
                            b.downcast_mut::<B>().expect("column holds type B"),
                            c.downcast_mut::<C>().expect("column holds type C"),
                        )
                    })
            })
    }

    /// Iterates over entities that have both A and B.
    pub fn query2<A: 'static, B: 'static>(&self) -> impl Iterator<Item = (Entity, &A, &B)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        self.archetypes
            .iter()
            .filter(move |arch| arch.contains(ta) && arch.contains(tb))
            .flat_map(move |arch| {
                arch.debug_assert_columns_aligned();
                let ca = arch
                    .columns
                    .get(&ta)
                    .expect("query2: archetype was filtered to contain column A");
                let cb = arch
                    .columns
                    .get(&tb)
                    .expect("query2: archetype was filtered to contain column B");
                arch.entities
                    .iter()
                    .zip(ca.iter())
                    .zip(cb.iter())
                    .map(|((&e, a), b)| {
                        (
                            e,
                            a.downcast_ref::<A>().expect("column holds type A"),
                            b.downcast_ref::<B>().expect("column holds type B"),
                        )
                    })
            })
    }

    /// Iterates over entities that have A, B, and C.
    pub fn query3<A: 'static, B: 'static, C: 'static>(
        &self,
    ) -> impl Iterator<Item = (Entity, &A, &B, &C)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        let tc = TypeId::of::<C>();
        self.archetypes
            .iter()
            .filter(move |arch| arch.contains(ta) && arch.contains(tb) && arch.contains(tc))
            .flat_map(move |arch| {
                arch.debug_assert_columns_aligned();
                let ca = arch
                    .columns
                    .get(&ta)
                    .expect("query3: archetype was filtered to contain column A");
                let cb = arch
                    .columns
                    .get(&tb)
                    .expect("query3: archetype was filtered to contain column B");
                let cc = arch
                    .columns
                    .get(&tc)
                    .expect("query3: archetype was filtered to contain column C");
                arch.entities
                    .iter()
                    .zip(ca.iter())
                    .zip(cb.iter())
                    .zip(cc.iter())
                    .map(|(((&e, a), b), c)| {
                        (
                            e,
                            a.downcast_ref::<A>().expect("column holds type A"),
                            b.downcast_ref::<B>().expect("column holds type B"),
                            c.downcast_ref::<C>().expect("column holds type C"),
                        )
                    })
            })
    }

    /// Iterates over entities that have A, B, C, and D.
    pub fn query4<A: 'static, B: 'static, C: 'static, D: 'static>(
        &self,
    ) -> impl Iterator<Item = (Entity, &A, &B, &C, &D)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        let tc = TypeId::of::<C>();
        let td = TypeId::of::<D>();
        self.archetypes
            .iter()
            .filter(move |arch| {
                arch.contains(ta) && arch.contains(tb) && arch.contains(tc) && arch.contains(td)
            })
            .flat_map(move |arch| {
                arch.debug_assert_columns_aligned();
                let ca = arch
                    .columns
                    .get(&ta)
                    .expect("query4: archetype was filtered to contain column A");
                let cb = arch
                    .columns
                    .get(&tb)
                    .expect("query4: archetype was filtered to contain column B");
                let cc = arch
                    .columns
                    .get(&tc)
                    .expect("query4: archetype was filtered to contain column C");
                let cd = arch
                    .columns
                    .get(&td)
                    .expect("query4: archetype was filtered to contain column D");
                arch.entities
                    .iter()
                    .zip(ca.iter())
                    .zip(cb.iter())
                    .zip(cc.iter())
                    .zip(cd.iter())
                    .map(|((((&e, a), b), c), d)| {
                        (
                            e,
                            a.downcast_ref::<A>().expect("column holds type A"),
                            b.downcast_ref::<B>().expect("column holds type B"),
                            c.downcast_ref::<C>().expect("column holds type C"),
                            d.downcast_ref::<D>().expect("column holds type D"),
                        )
                    })
            })
    }

    /// Iterates over entities that have A and also have B.
    pub fn query_with<A: 'static, B: 'static>(&self) -> impl Iterator<Item = (Entity, &A)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        self.archetypes
            .iter()
            .filter(move |arch| arch.contains(ta) && arch.contains(tb))
            .flat_map(move |arch| {
                arch.debug_assert_columns_aligned();
                let col = arch
                    .columns
                    .get(&ta)
                    .expect("query_with: archetype was filtered to contain column A");
                arch.entities
                    .iter()
                    .zip(col.iter())
                    .map(|(&e, c)| (e, c.downcast_ref::<A>().expect("column holds type A")))
            })
    }

    /// Iterates over entities that have A but do not have B.
    pub fn query_without<A: 'static, B: 'static>(&self) -> impl Iterator<Item = (Entity, &A)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        self.archetypes
            .iter()
            .filter(move |arch| arch.contains(ta) && !arch.contains(tb))
            .flat_map(move |arch| {
                arch.debug_assert_columns_aligned();
                let col = arch
                    .columns
                    .get(&ta)
                    .expect("query_without: archetype was filtered to contain column A");
                arch.entities
                    .iter()
                    .zip(col.iter())
                    .map(|(&e, c)| (e, c.downcast_ref::<A>().expect("column holds type A")))
            })
    }

    /// Iterates over all entities with A. B is Some if present, None otherwise.
    pub fn query_opt2<A: 'static, B: 'static>(
        &self,
    ) -> impl Iterator<Item = (Entity, &A, Option<&B>)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        self.archetypes
            .iter()
            .filter(move |arch| arch.contains(ta))
            .flat_map(move |arch| {
                arch.debug_assert_columns_aligned();
                let ca = arch
                    .columns
                    .get(&ta)
                    .expect("query_opt2: archetype was filtered to contain column A");
                let cb = arch.columns.get(&tb);
                // Only the mandatory pair can zip: `B` is optional, so its column is an
                // `Option<&Vec<_>>` with no per-element iterator to zip against, and the row index
                // is the one way to reach it. Zipping A anyway drops one bounds check of the two.
                arch.entities
                    .iter()
                    .zip(ca.iter())
                    .enumerate()
                    .map(move |(i, (&e, a))| {
                        let a = a.downcast_ref::<A>().expect("column holds type A");
                        let b =
                            cb.map(|col| col[i].downcast_ref::<B>().expect("column holds type B"));
                        (e, a, b)
                    })
            })
    }
}
