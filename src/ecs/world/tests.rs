use super::*;

#[allow(dead_code)]
struct Position {
    x: f32,
    y: f32,
}
#[allow(dead_code)]
struct Health(u32);
#[allow(dead_code)]
struct Velocity {
    vx: f32,
    vy: f32,
}

#[test]
fn spawn_and_query() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Position { x: 1.0, y: 2.0 });
    world.add_component(e, Health(100));

    let pos = world.get::<Position>(e).unwrap();
    assert_eq!(pos.x, 1.0);

    let count = world.query::<Position>().count();
    assert_eq!(count, 1);
}

#[test]
fn query_mut_mutates_all_matching() {
    let mut world = World::new();
    let e0 = world.spawn();
    let e1 = world.spawn();
    world.add_component(e0, Position { x: 1.0, y: 2.0 });
    world.add_component(e1, Position { x: 3.0, y: 4.0 });

    // Mutably iterate and modify all Positions at once via query_mut (no collect+get_mut workaround needed)
    for (_e, p) in world.query_mut::<Position>() {
        p.x += 10.0;
    }

    assert_eq!(world.get::<Position>(e0).unwrap().x, 11.0);
    assert_eq!(world.get::<Position>(e1).unwrap().x, 13.0);
}

#[test]
fn resource() {
    let mut world = World::new();
    world.insert_resource(42u32);
    assert_eq!(*world.resource::<u32>().unwrap(), 42);
}

#[test]
fn despawn_removes_entity_from_query() {
    let mut world = World::new();
    let e0 = world.spawn();
    let e1 = world.spawn();
    let e2 = world.spawn();
    world.add_component(e0, Position { x: 0.0, y: 0.0 });
    world.add_component(e1, Position { x: 1.0, y: 0.0 });
    world.add_component(e2, Position { x: 2.0, y: 0.0 });

    world.despawn(e1);

    let positions: Vec<_> = world.query::<Position>().collect();
    assert_eq!(positions.len(), 2);
    assert!(positions.iter().all(|(e, _)| *e != e1));
}

#[test]
fn despawn_is_idempotent() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Health(50));
    world.despawn(e);
    world.despawn(e);
}

#[test]
fn spawn_reuses_freed_index_with_incremented_generation() {
    let mut world = World::new();
    let e_first = world.spawn();
    world.add_component(e_first, Health(99));

    world.despawn(e_first);

    let e_second = world.spawn();
    assert_eq!(e_first.index(), e_second.index());
    assert_eq!(e_first.generation() + 1, e_second.generation());
    assert!(!world.is_alive(e_first));
    assert!(world.is_alive(e_second));
    assert!(world.get::<Health>(e_second).is_none());
}

#[test]
fn query2_returns_only_entities_with_both() {
    let mut world = World::new();

    let ea = world.spawn();
    world.add_component(ea, Position { x: 0.0, y: 0.0 });

    let eb = world.spawn();
    world.add_component(eb, Health(10));

    let eab = world.spawn();
    world.add_component(eab, Position { x: 1.0, y: 1.0 });
    world.add_component(eab, Health(20));

    let results: Vec<_> = world.query2::<Position, Health>().collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, eab);
}

#[test]
fn query2_mut_mutates_both_components() {
    let mut world = World::new();
    let e0 = world.spawn();
    world.add_component(e0, Position { x: 0.0, y: 0.0 });
    world.add_component(e0, Velocity { vx: 1.0, vy: 2.0 });
    // Entity with only Position must be skipped.
    let e1 = world.spawn();
    world.add_component(e1, Position { x: 5.0, y: 5.0 });

    let mut visited = 0;
    for (_e, p, v) in world.query2_mut::<Position, Velocity>() {
        p.x += v.vx;
        p.y += v.vy;
        v.vx = 0.0; // mutate the second component too
        visited += 1;
    }
    assert_eq!(
        visited, 1,
        "only the entity with both components is visited"
    );

    let p = world.get::<Position>(e0).unwrap();
    assert_eq!((p.x, p.y), (1.0, 2.0));
    assert_eq!(world.get::<Velocity>(e0).unwrap().vx, 0.0);
    assert_eq!(world.get::<Position>(e1).unwrap().x, 5.0); // untouched
}

#[test]
fn remove_component_keeps_entity_alive() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Position { x: 1.0, y: 2.0 });
    world.add_component(e, Health(50));

    world.remove_component::<Position>(e);

    assert!(world.get::<Position>(e).is_none());
    assert_eq!(world.get::<Health>(e).unwrap().0, 50);
    assert!(world.entities().contains(&e));
}

#[test]
fn remove_component_nonexistent_is_noop() {
    let mut world = World::new();
    let e = world.spawn();
    world.remove_component::<Position>(e);
    world.add_component(e, Health(10));
    world.remove_component::<Velocity>(e);
    assert_eq!(world.get::<Health>(e).unwrap().0, 10);
}

#[test]
fn query4_returns_only_entities_with_all_four() {
    struct Tag;

    let mut world = World::new();

    let eabc = world.spawn();
    world.add_component(eabc, Position { x: 0.0, y: 0.0 });
    world.add_component(eabc, Health(1));
    world.add_component(eabc, Velocity { vx: 0.0, vy: 0.0 });

    let eabcd = world.spawn();
    world.add_component(eabcd, Position { x: 1.0, y: 1.0 });
    world.add_component(eabcd, Health(2));
    world.add_component(eabcd, Velocity { vx: 1.0, vy: 0.0 });
    world.add_component(eabcd, Tag);

    let results: Vec<_> = world.query4::<Position, Health, Velocity, Tag>().collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, eabcd);
}

#[test]
fn query_opt2_returns_a_with_optional_b() {
    let mut world = World::new();

    let ea = world.spawn();
    world.add_component(ea, Position { x: 1.0, y: 0.0 });

    let eab = world.spawn();
    world.add_component(eab, Position { x: 2.0, y: 0.0 });
    world.add_component(eab, Health(50));

    let eb = world.spawn();
    world.add_component(eb, Health(99));

    let results: Vec<_> = world.query_opt2::<Position, Health>().collect();
    assert_eq!(results.len(), 2);

    let ea_result = results.iter().find(|(e, _, _)| *e == ea).unwrap();
    assert!(ea_result.2.is_none());

    let eab_result = results.iter().find(|(e, _, _)| *e == eab).unwrap();
    assert_eq!(eab_result.2.unwrap().0, 50);
}

#[test]
fn query3_returns_only_entities_with_all_three() {
    let mut world = World::new();

    let eab = world.spawn();
    world.add_component(eab, Position { x: 0.0, y: 0.0 });
    world.add_component(eab, Health(5));

    let eabc = world.spawn();
    world.add_component(eabc, Position { x: 1.0, y: 1.0 });
    world.add_component(eabc, Health(10));
    world.add_component(eabc, Velocity { vx: 1.0, vy: 0.0 });

    let results: Vec<_> = world.query3::<Position, Health, Velocity>().collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, eabc);
}

#[test]
fn query_with_returns_only_entities_with_both() {
    let mut world = World::new();

    let e_pos_only = world.spawn();
    world.add_component(e_pos_only, Position { x: 0.0, y: 0.0 });

    let e_health_only = world.spawn();
    world.add_component(e_health_only, Health(10));

    let e_both = world.spawn();
    world.add_component(e_both, Position { x: 1.0, y: 1.0 });
    world.add_component(e_both, Health(50));

    let results: Vec<_> = world.query_with::<Position, Health>().collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, e_both);
    assert_eq!(results[0].1.x, 1.0);
}

#[test]
fn query_without_returns_only_entities_lacking_filter() {
    let mut world = World::new();

    let e_pos_only = world.spawn();
    world.add_component(e_pos_only, Position { x: 5.0, y: 0.0 });

    let e_both = world.spawn();
    world.add_component(e_both, Position { x: 2.0, y: 0.0 });
    world.add_component(e_both, Health(30));

    let results: Vec<_> = world.query_without::<Position, Health>().collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, e_pos_only);
    assert_eq!(results[0].1.x, 5.0);
}

#[test]
fn query_with_and_without_mixed_entities() {
    let mut world = World::new();

    // has Position + Health + Velocity
    let e_all = world.spawn();
    world.add_component(e_all, Position { x: 1.0, y: 0.0 });
    world.add_component(e_all, Health(100));
    world.add_component(e_all, Velocity { vx: 1.0, vy: 0.0 });

    // has Position + Health (no Velocity)
    let e_ph = world.spawn();
    world.add_component(e_ph, Position { x: 2.0, y: 0.0 });
    world.add_component(e_ph, Health(50));

    // has Position only
    let e_p = world.spawn();
    world.add_component(e_p, Position { x: 3.0, y: 0.0 });

    // has Health only
    let e_h = world.spawn();
    world.add_component(e_h, Health(10));

    // query_with::<Position, Health> → e_all, e_ph
    let with_results: Vec<Entity> = world
        .query_with::<Position, Health>()
        .map(|(e, _)| e)
        .collect();
    assert_eq!(with_results.len(), 2);
    assert!(with_results.contains(&e_all));
    assert!(with_results.contains(&e_ph));

    // query_without::<Position, Health> → e_p
    let without_results: Vec<Entity> = world
        .query_without::<Position, Health>()
        .map(|(e, _)| e)
        .collect();
    assert_eq!(without_results.len(), 1);
    assert_eq!(without_results[0], e_p);

    // query_with::<Position, Velocity> → e_all only
    let with_vel: Vec<Entity> = world
        .query_with::<Position, Velocity>()
        .map(|(e, _)| e)
        .collect();
    assert_eq!(with_vel.len(), 1);
    assert_eq!(with_vel[0], e_all);
}

#[test]
fn archetype_reuse_across_entities() {
    let mut world = World::new();

    let e1 = world.spawn();
    world.add_component(e1, Position { x: 1.0, y: 0.0 });
    world.add_component(e1, Health(10));

    let e2 = world.spawn();
    world.add_component(e2, Position { x: 2.0, y: 0.0 });
    world.add_component(e2, Health(20));

    // Same component set → should be placed in the same Archetype
    let (arch1, _) = world.entity_location[&e1];
    let (arch2, _) = world.entity_location[&e2];
    assert_eq!(arch1, arch2);
    assert_eq!(world.query2::<Position, Health>().count(), 2);
}

#[test]
fn add_component_replaces_existing() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Health(10));
    world.add_component(e, Health(99)); // replace
    assert_eq!(world.get::<Health>(e).unwrap().0, 99);
    assert_eq!(world.query::<Health>().count(), 1);
}

#[test]
fn change_tracking_added() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, 42u32);
    let added: Vec<_> = world.query_added::<u32>().collect();
    assert_eq!(added.len(), 1);
    assert_eq!(*added[0].1, 42);
    // should not appear in changed
    assert_eq!(world.query_changed::<u32>().count(), 0);
    // gone after clear
    world.clear_change_tracking();
    assert_eq!(world.query_added::<u32>().count(), 0);
}

#[test]
fn change_tracking_changed() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, 1u32);
    world.clear_change_tracking();
    // replace
    world.add_component(e, 2u32);
    assert_eq!(world.query_added::<u32>().count(), 0);
    let changed: Vec<_> = world.query_changed::<u32>().collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(*changed[0].1, 2);
}

#[test]
fn mark_changed_tracks_direct_mutation() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, 1u32);
    world.clear_change_tracking();

    *world.get_mut::<u32>(e).unwrap() = 7;
    assert_eq!(world.query_changed::<u32>().count(), 0);

    assert!(world.mark_changed::<u32>(e));
    let changed: Vec<_> = world.query_changed::<u32>().collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(*changed[0].1, 7);
}

#[test]
fn get_mut_tracked_marks_changed() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, 1u32);
    world.clear_change_tracking();

    *world.get_mut_tracked::<u32>(e).unwrap() = 3;

    let changed: Vec<_> = world.query_changed::<u32>().collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(*changed[0].1, 3);
}

#[test]
fn mark_changed_missing_component_returns_false() {
    let mut world = World::new();
    let e = world.spawn();

    assert!(!world.mark_changed::<u32>(e));
    assert!(world.get_mut_tracked::<u32>(e).is_none());
}

#[test]
fn change_tracking_despawn_clears() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, 99u32);
    assert_eq!(world.query_added::<u32>().count(), 1);
    world.despawn(e);
    assert_eq!(world.query_added::<u32>().count(), 0);
}

#[test]
fn change_tracking_remove_clears() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, 7u32);
    assert_eq!(world.query_added::<u32>().count(), 1);
    world.remove_component::<u32>(e);
    assert_eq!(world.query_added::<u32>().count(), 0);
}

#[test]
fn clone_entity_copies_components() {
    let mut world = World::new();
    world.register_clone::<u32>();
    world.register_clone::<f32>();

    let src = world.spawn();
    world.add_component(src, 42u32);
    world.add_component(src, 3.125f32);

    let dst = world.clone_entity(src).unwrap();
    assert_ne!(src, dst);
    assert_eq!(*world.get::<u32>(dst).unwrap(), 42);
    assert!((world.get::<f32>(dst).unwrap() - 3.125).abs() < 1e-5);
    // src still intact
    assert_eq!(*world.get::<u32>(src).unwrap(), 42);
}

#[test]
fn clone_entity_skips_unregistered_types() {
    #[derive(Debug)]
    struct NotCloneable;
    unsafe impl Send for NotCloneable {}
    unsafe impl Sync for NotCloneable {}

    let mut world = World::new();
    world.register_clone::<u32>();

    let src = world.spawn();
    world.add_component(src, 99u32);
    world.add_component(src, NotCloneable);

    let dst = world.clone_entity(src).unwrap();
    assert_eq!(*world.get::<u32>(dst).unwrap(), 99);
    assert!(world.get::<NotCloneable>(dst).is_none()); // not copied
}

#[test]
fn clone_entity_dead_src_returns_none() {
    let mut world = World::new();
    let src = world.spawn();
    world.despawn(src);

    let dst = world.clone_entity(src);
    assert_eq!(dst, None);
    assert_eq!(world.entity_count(), 0);
}

#[test]
fn stale_handle_cannot_mutate_reused_entity() {
    let mut world = World::new();
    let stale = world.spawn();
    world.add_component(stale, Health(10));
    world.despawn(stale);

    let reused = world.spawn();
    assert_eq!(stale.index(), reused.index());
    assert_ne!(stale.generation(), reused.generation());

    assert!(!world.is_alive(stale));
    assert!(world.get::<Health>(stale).is_none());
    assert!(world.get_mut::<Health>(stale).is_none());

    world.add_component(stale, Health(99));
    assert!(world.get::<Health>(reused).is_none());

    world.add_component(reused, Health(5));
    world.remove_component::<Health>(stale);
    assert_eq!(world.get::<Health>(reused).unwrap().0, 5);

    assert!(!world.mark_changed::<Health>(stale));
    assert!(world.take_component::<Health>(stale).is_none());

    world.despawn(stale);
    assert!(world.is_alive(reused));
}

#[test]
fn commands_ignore_stale_handles() {
    let mut world = World::new();
    let stale = world.spawn();
    world.despawn(stale);
    let reused = world.spawn();

    let mut cmds = crate::ecs::Commands::new();
    cmds.insert(stale, Health(77));
    cmds.remove::<Health>(stale);
    cmds.despawn(stale);
    world.apply_commands(cmds);

    assert!(world.is_alive(reused));
    assert!(world.get::<Health>(reused).is_none());
}

#[test]
fn world_remove_resource_removes_and_returns() {
    let mut world = World::new();
    world.insert_resource(42u32);
    assert_eq!(world.resource::<u32>().copied(), Some(42));

    let v = world.remove_resource::<u32>();
    assert_eq!(v, Some(42));
    assert_eq!(world.resource::<u32>(), None);
}

#[test]
fn world_remove_resource_missing_returns_none() {
    let mut world = World::new();
    let v = world.remove_resource::<u32>();
    assert_eq!(v, None);
}

#[test]
fn erased_resource_roundtrips_between_worlds() {
    // Core step of the persistent-resource mechanism (App::register_persistent):
    // a value extracted as a type-erased box and inserted into a new World is preserved.
    #[derive(Debug, PartialEq)]
    struct Score(u32);

    let tid = TypeId::of::<Score>();
    let mut old = World::new();
    old.insert_resource(Score(42));

    let boxed = old
        .take_resource_erased(tid)
        .expect("resource should exist");
    assert!(
        old.resource::<Score>().is_none(),
        "old World should not have the resource after it was taken"
    );

    let mut fresh = World::new();
    fresh.insert_resource_erased(tid, boxed);
    assert_eq!(fresh.resource::<Score>(), Some(&Score(42)));
}

#[test]
fn take_resource_erased_missing_returns_none() {
    let mut world = World::new();
    assert!(world.take_resource_erased(TypeId::of::<u32>()).is_none());
}

/// Mass-despawn removes all change-tracking entries for despawned entities in O(1)
/// per entity. After despawning all entities, `query_added` and `query_changed`
/// must return empty results (no stale entries).
#[test]
fn mass_despawn_clears_change_tracking() {
    let mut world = World::new();

    // Spawn 64 entities and add/replace components to populate both sets.
    let entities: Vec<Entity> = (0..64).map(|_| world.spawn()).collect();
    for &e in &entities {
        world.add_component(e, Position { x: 1.0, y: 2.0 });
    }
    // Replace to also populate `changed_this_tick`.
    for &e in &entities {
        world.add_component(e, Position { x: 3.0, y: 4.0 });
    }

    assert!(world.query_added::<Position>().count() > 0);
    assert!(world.query_changed::<Position>().count() > 0);

    // Mass-despawn — O(1) per entity thanks to HashMap keyed by Entity.
    for e in entities {
        world.despawn(e);
    }

    // No stale tracking entries should remain.
    assert_eq!(world.query_added::<Position>().count(), 0);
    assert_eq!(world.query_changed::<Position>().count(), 0);
}

/// Change-tracking queries return the same results before and after the
/// HashSet<(Entity,TypeId)> → HashMap<Entity,HashSet<TypeId>> restructure:
/// added/changed sets are disjoint for a given component in one tick.
#[test]
fn change_tracking_restructure_semantics_preserved() {
    let mut world = World::new();

    let e_new = world.spawn();
    world.add_component(e_new, Health(10));

    let e_old = world.spawn();
    world.add_component(e_old, Health(5));
    world.clear_change_tracking();

    // Replace on e_old → changed; e_new still in added set for previous tick
    // (cleared above), so now e_new added → nothing since we cleared.
    // Add a fresh entity in this tick to be "added".
    let e_added_now = world.spawn();
    world.add_component(e_added_now, Health(99));

    // Replace e_old's Health → changed.
    world.add_component(e_old, Health(77));

    let added: Vec<_> = world.query_added::<Health>().collect();
    let changed: Vec<_> = world.query_changed::<Health>().collect();

    assert_eq!(added.len(), 1);
    assert_eq!(added[0].0, e_added_now);
    assert_eq!(added[0].1 .0, 99);

    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].0, e_old);
    assert_eq!(changed[0].1 .0, 77);
}

#[test]
fn with_resource_mut_mutation_persists() {
    let mut world = World::new();
    world.insert_resource(10u32);

    let called = world.with_resource_mut::<u32, _>(|r, _w| {
        *r = 42;
    });

    assert!(
        called,
        "with_resource_mut must return true when R is present"
    );
    assert_eq!(
        world.resource::<u32>().copied(),
        Some(42),
        "mutation inside the closure must persist after re-insertion"
    );
}

#[test]
fn with_resource_mut_world_access_inside_closure() {
    // The closure receives &mut World with R removed, so other world state is accessible.
    let mut world = World::new();
    world.insert_resource(1u32);
    world.insert_resource(99i32); // a second resource

    world.with_resource_mut::<u32, _>(|r, w| {
        // u32 is removed, but i32 is still reachable.
        let other = w.resource_mut::<i32>().unwrap();
        *other = 7;
        *r += 1;
        // Spawning an entity inside the closure also works.
        w.spawn();
    });

    assert_eq!(world.resource::<u32>().copied(), Some(2));
    assert_eq!(world.resource::<i32>().copied(), Some(7));
    assert_eq!(world.entity_count(), 1);
}

#[test]
fn with_resource_mut_absent_returns_false_without_calling_f() {
    let mut world = World::new();
    // u32 is never inserted.
    let mut called = false;
    let result = world.with_resource_mut::<u32, _>(|_, _| {
        called = true;
    });

    assert!(!result, "must return false when R is absent");
    assert!(!called, "closure must not be invoked when R is absent");
}

#[test]
fn has_component_returns_true_when_present() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Health(10));
    assert!(world.has_component::<Health>(e));
    assert!(!world.has_component::<Position>(e));
}

#[test]
fn has_component_returns_false_for_dead_entity() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Health(5));
    world.despawn(e);
    assert!(!world.has_component::<Health>(e));
}

/// `query_added` and `query_changed` yield no entities when their tracking sets
/// are empty (the common inter-frame case). The early-return path must not panic
/// or allocate.
#[test]
fn query_added_empty_tracking_yields_nothing() {
    let world = World::new(); // fresh world — no adds yet
    assert_eq!(world.query_added::<u32>().count(), 0);
}

#[test]
fn query_changed_empty_tracking_yields_nothing() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, 1u32);
    // Clear tracking to simulate start of a new frame.
    world.clear_change_tracking();
    assert_eq!(world.query_changed::<u32>().count(), 0);
}

/// The take → mutate → put-back idiom is what `BehaviorSystem` (`src/behavior.rs:401`) and
/// `TimelineSystem` (`src/timeline.rs:303`) run every frame, so it must report a **change**
/// — on every frame, not just the first.
///
/// Before this was fixed, `take_component`'s internal `remove_component` cleared both tracking
/// sets and the put-back then took `add_component`'s new-component branch, so `query_added`
/// yielded every ticked entity on every single frame and `query_changed` yielded none.
#[test]
fn take_then_readd_reports_changed_not_added() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Health(1));

    for frame in 0..3 {
        world.clear_change_tracking(); // App does this at the start of every frame
        let mut h = world
            .take_component::<Health>(e)
            .expect("component is present");
        h.0 += 1;
        world.add_component(e, h);

        assert_eq!(
            world.query_added::<Health>().count(),
            0,
            "frame {frame}: a put-back component was already there — it is not newly added"
        );
        let changed: Vec<_> = world.query_changed::<Health>().collect();
        assert_eq!(changed.len(), 1, "frame {frame}: the put-back is a change");
        assert_eq!(changed[0].0, e);
    }
    assert_eq!(
        world.get::<Health>(e).unwrap().0,
        4,
        "all three ticks landed"
    );
}

/// A component that is genuinely new *this* tick keeps its "added" status across a
/// take → put-back round trip rather than being downgraded to "changed".
#[test]
fn take_then_readd_keeps_added_for_a_component_added_this_tick() {
    let mut world = World::new();
    let e = world.spawn();
    world.clear_change_tracking();

    world.add_component(e, Health(1)); // first appearance, this tick
    let h = world.take_component::<Health>(e).unwrap();
    world.add_component(e, h);

    assert_eq!(world.query_added::<Health>().count(), 1);
    assert_eq!(world.query_changed::<Health>().count(), 0);
}

/// `take_component` with no put-back leaves no phantom entry: the tracked TypeId survives
/// until the next `clear_change_tracking`, but both queries filter it out because the
/// component is genuinely gone.
#[test]
fn take_without_readd_reports_neither_added_nor_changed() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Health(1));
    world.clear_change_tracking();

    assert!(world.take_component::<Health>(e).is_some());
    assert_eq!(world.query_added::<Health>().count(), 0);
    assert_eq!(world.query_changed::<Health>().count(), 0);
    assert!(world.get::<Health>(e).is_none());
}

/// `with_resource_mut` must not take the resource down with an unwind.
///
/// `App`'s default `SystemPanicPolicy::DisableSystemAndContinue` catches a system panic and
/// keeps the frame loop running, so before this was fixed the session simply carried on with
/// the resource permanently missing — and the only log line named the panicking system, never
/// the resource that vanished.
#[test]
fn with_resource_mut_restores_the_resource_when_the_closure_panics() {
    let mut world = World::new();
    world.insert_resource(Health(7));

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.with_resource_mut::<Health, _>(|_h, _w| panic!("system blew up"));
    }));

    assert!(outcome.is_err(), "the panic must still reach the caller");
    assert_eq!(
        world.resource::<Health>().map(|h| h.0),
        Some(7),
        "the resource belongs back in the World, not dropped on the unwind path"
    );
}

/// Inserting a fresh `R` through the closure's `&mut World` — a config/registry hot-reload,
/// which is what that argument exists for — must win over the value taken at entry.
#[test]
fn with_resource_mut_keeps_a_replacement_inserted_by_the_closure() {
    let mut world = World::new();
    world.insert_resource(Health(1));

    assert!(world.with_resource_mut::<Health, _>(|_old, w| {
        w.insert_resource(Health(999));
    }));

    assert_eq!(
        world.resource::<Health>().unwrap().0,
        999,
        "the unconditional re-insert used to clobber the reload back to the stale value"
    );
}

/// A panic inside a registered clone closure must not unregister the type.
///
/// The registry used to hand the closure out by removing it from the map for the duration of
/// the call, so the first panic lost the registration and every later `clone_entity` silently
/// copied nothing. The second iteration is the assertion: it can only panic again if the
/// registration survived the first one.
#[test]
fn clone_registration_survives_a_panic_in_the_clone_closure() {
    struct PanicOnClone;
    impl Clone for PanicOnClone {
        fn clone(&self) -> Self {
            panic!("clone blew up")
        }
    }

    let mut world = World::new();
    world.register_clone::<PanicOnClone>();
    let src = world.spawn();
    world.add_component(src, PanicOnClone);

    for attempt in 0..2 {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            world.clone_entity(src);
        }));
        assert!(
            outcome.is_err(),
            "attempt {attempt}: the registration must still be in place — the old \
             remove → call → re-insert lost it on the first panic, after which this \
             clone would silently copy nothing and report success"
        );
    }
}

/// `query_added` / `query_changed` collect from a `HashMap<Entity, _>`, so their order used to
/// be seeded per process — the only public queries in the crate that were not reproducible.
/// They now yield ascending `(index, generation)`.
#[test]
fn change_tracking_queries_yield_entities_in_index_order() {
    let mut world = World::new();
    let spawned: Vec<Entity> = (0..8).map(|_| world.spawn()).collect();
    for &e in &spawned {
        world.add_component(e, Health(e.index()));
    }

    let added: Vec<u32> = world
        .query_added::<Health>()
        .map(|(e, _)| e.index())
        .collect();
    assert_eq!(
        added,
        (0..8).collect::<Vec<_>>(),
        "ascending, not hash order"
    );

    world.clear_change_tracking();
    for &e in &spawned {
        world.add_component(e, Health(e.index() * 10)); // replace → changed
    }
    let changed: Vec<u32> = world
        .query_changed::<Health>()
        .map(|(e, _)| e.index())
        .collect();
    assert_eq!(changed, (0..8).collect::<Vec<_>>());
}

/// `clone_entity` walks `clone_registry.keys()`, and every `add_component` in that loop moves
/// the entity through a different intermediate archetype signature — so an unsorted order left
/// `world.archetypes` in a different Vec order per run, and `query::<T>()` iterates archetypes
/// in exactly that order.
///
/// Each `HashMap` gets its own seed, so building the identical world several times *in one
/// process* is enough to catch it: the unsorted version disagrees with itself here (measured
/// 18/6 across 24 process launches before the fix), while the sorted one cannot.
#[test]
fn clone_entity_leaves_the_same_query_order_every_time() {
    #[derive(Clone)]
    struct C1(u32);
    #[derive(Clone)]
    struct C2;
    #[derive(Clone)]
    struct C3;
    #[derive(Clone)]
    struct C4;

    fn build() -> Vec<u32> {
        let mut world = World::new();
        world.register_clone::<C1>();
        world.register_clone::<C2>();
        world.register_clone::<C3>();
        world.register_clone::<C4>();

        // Built back-to-front so the clone's intermediate signatures are genuinely new
        // archetypes rather than ones this build already created.
        let src = world.spawn();
        world.add_component(src, C4);
        world.add_component(src, C3);
        world.add_component(src, C2);
        world.add_component(src, C1(0));
        world.clone_entity(src).expect("src is alive");

        let x = world.spawn();
        world.add_component(x, C1(10));
        world.add_component(x, C2);
        let y = world.spawn();
        world.add_component(y, C1(20));
        world.add_component(y, C3);

        world.query::<C1>().map(|(_, c)| c.0).collect()
    }

    let first = build();
    for attempt in 1..16 {
        assert_eq!(
            build(),
            first,
            "attempt {attempt}: clone_entity must leave one archetype order, not a per-map one"
        );
    }
}
