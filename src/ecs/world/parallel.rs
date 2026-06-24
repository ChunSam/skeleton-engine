//! Rayon-backed parallel queries (native only — WASM is single-threaded).
//!
//! The enclosing `mod parallel;` declaration in `world.rs` is itself
//! `#[cfg(not(target_arch = "wasm32"))]`, so this whole file is absent on wasm.

use super::{Entity, World};
use std::any::TypeId;

impl World {
    /// Applies a closure **in parallel** to all entities with T (read-only).
    ///
    /// To collect results, use a `Mutex` or channel inside the closure,
    /// or use `par_query_map` if you need return values.
    ///
    /// ```text
    /// world.par_query_for_each::<Transform, _>(|e, t| {
    ///     println!("{e:?} pos={}", t.position);
    /// });
    /// ```
    pub fn par_query_for_each<T, F>(&self, f: F)
    where
        T: Send + Sync + 'static,
        F: Fn(Entity, &T) + Send + Sync,
    {
        use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
        let tid = TypeId::of::<T>();
        self.archetypes
            .par_iter()
            .filter(|arch| arch.contains(tid))
            .for_each(|arch| {
                let col = arch
                    .columns
                    .get(&tid)
                    .expect("par_query_for_each: archetype was filtered to contain this column");
                arch.entities
                    .par_iter()
                    .zip(col.par_iter())
                    .for_each(|(&e, c)| f(e, c.downcast_ref::<T>().expect("column holds type T")));
            });
    }

    /// Applies a mapping closure **in parallel** to all entities with T and returns the results as `Vec<R>`.
    ///
    /// ```text
    /// let positions: Vec<(Entity, Vec2)> =
    ///     world.par_query_map::<Transform, _, _>(|e, t| (e, t.position));
    /// ```
    pub fn par_query_map<T, R, F>(&self, f: F) -> Vec<R>
    where
        T: Send + Sync + 'static,
        R: Send,
        F: Fn(Entity, &T) -> R + Send + Sync,
    {
        use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
        let tid = TypeId::of::<T>();
        self.archetypes
            .par_iter()
            .filter(|arch| arch.contains(tid))
            .flat_map(|arch| {
                let col = arch
                    .columns
                    .get(&tid)
                    .expect("par_query_map: archetype was filtered to contain this column");
                arch.entities
                    .par_iter()
                    .zip(col.par_iter())
                    .map(|(&e, c)| f(e, c.downcast_ref::<T>().expect("column holds type T")))
            })
            .collect()
    }

    /// Applies a closure **in parallel** to all entities with both A and B (read-only).
    pub fn par_query2_for_each<A, B, F>(&self, f: F)
    where
        A: Send + Sync + 'static,
        B: Send + Sync + 'static,
        F: Fn(Entity, &A, &B) + Send + Sync,
    {
        use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        self.archetypes
            .par_iter()
            .filter(move |arch| arch.contains(ta) && arch.contains(tb))
            .for_each(|arch| {
                let ca = arch
                    .columns
                    .get(&ta)
                    .expect("par_query2_for_each: archetype was filtered to contain column A");
                let cb = arch
                    .columns
                    .get(&tb)
                    .expect("par_query2_for_each: archetype was filtered to contain column B");
                arch.entities
                    .par_iter()
                    .zip(ca.par_iter())
                    .zip(cb.par_iter())
                    .for_each(|((&e, a), b)| {
                        f(
                            e,
                            a.downcast_ref::<A>().expect("column holds type A"),
                            b.downcast_ref::<B>().expect("column holds type B"),
                        );
                    });
            });
    }

    /// Applies a mapping closure **in parallel** to all entities with both A and B and returns the results as `Vec<R>`.
    pub fn par_query2_map<A, B, R, F>(&self, f: F) -> Vec<R>
    where
        A: Send + Sync + 'static,
        B: Send + Sync + 'static,
        R: Send,
        F: Fn(Entity, &A, &B) -> R + Send + Sync,
    {
        use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        self.archetypes
            .par_iter()
            .filter(move |arch| arch.contains(ta) && arch.contains(tb))
            .flat_map(|arch| {
                let ca = arch
                    .columns
                    .get(&ta)
                    .expect("par_query2_map: archetype was filtered to contain column A");
                let cb = arch
                    .columns
                    .get(&tb)
                    .expect("par_query2_map: archetype was filtered to contain column B");
                arch.entities
                    .par_iter()
                    .zip(ca.par_iter())
                    .zip(cb.par_iter())
                    .map(|((&e, a), b)| {
                        f(
                            e,
                            a.downcast_ref::<A>().expect("column holds type A"),
                            b.downcast_ref::<B>().expect("column holds type B"),
                        )
                    })
            })
            .collect()
    }
}
