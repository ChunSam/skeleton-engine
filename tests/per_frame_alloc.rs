//! Per-frame allocation measurements for built-in systems.
//!
//! **Its OWN integration binary, and it must stay that way**: it installs a
//! `#[global_allocator]`, and a process gets exactly one. Do not merge these tests into another
//! file, and do not add unrelated tests here — every allocation in this binary is counted.
//!
//! # Why this exists
//!
//! The engine's rule is that a system running every frame keeps its temporaries as scratch fields
//! (`clear()` + refill) or uses `std::mem::take`. Until now that rule was enforced by **reading
//! the code**, which is how the 2026-08-07 analysis produced a list of per-frame allocation
//! candidates that nobody could confirm or refute without reading them all again. Six were fixed
//! in v0.150.0 on exactly that basis — plausible, unmeasured.
//!
//! These tests measure instead. Each drives a system for two frames over an unchanging World and
//! asserts the **second** frame allocates nothing: the first frame is allowed to fill scratch
//! buffers, and steady state is the property that actually matters for a game loop.
//!
//! # Reading a failure
//!
//! A failure prints allocation count and bytes for the steady-state frame. That is a real
//! regression: something in that system started allocating every frame. Find it by bisecting the
//! system's body with `measure()`.

use std::alloc::{GlobalAlloc, Layout, System as SysAlloc};
use std::cell::Cell;

use engine::{
    Collider, CollisionLayer, Color, HierarchySystem, Label, LayoutDir, LayoutSystem,
    LocaleResource, LocalizationSystem, LocalizedText, Panel, Parent, Particle, ParticleSystem,
    SpatialGrid, Sprite, System, TilemapSystem, Transform, UiNode, Vec2, World,
};

// ── Counting allocator ────────────────────────────────────────────────────────────────────────

// Counters are **thread-local**, not global. The first version of this file used globals plus a
// mutex around `measure`, and its own control test caught the flaw: the mutex serialises the
// measuring calls, but every OTHER test thread keeps allocating in its setup while the counter is
// armed, and those allocations land in whoever is measuring. Per-thread counters make a
// measurement see only its own work, and remove the need for the mutex entirely.
//
// `const`-initialised so the TLS needs no lazy allocation on first touch — a `thread_local!` that
// allocates while being read from inside the allocator would recurse. `Cell<usize>` has no
// destructor, so there is no destructor registration to allocate either.
thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
    static COUNTING: Cell<bool> = const { Cell::new(false) };
}

struct Counting;

fn record(size: usize) {
    // `try_with`: during thread teardown the TLS may already be gone, and an allocation then must
    // not panic inside the allocator.
    let _ = COUNTING.try_with(|c| {
        if c.get() {
            let _ = ALLOCS.try_with(|a| a.set(a.get() + 1));
            let _ = BYTES.try_with(|b| b.set(b.get() + size));
        }
    });
}

// SAFETY: every method forwards to the system allocator unchanged; the counters are the only
// addition and they never touch the returned pointers.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        SysAlloc.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        SysAlloc.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // A grow counts: a scratch buffer that keeps growing every frame is exactly the bug these
        // tests exist to catch, and it would be invisible if only `alloc` were counted.
        if new_size > layout.size() {
            record(new_size - layout.size());
        }
        SysAlloc.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Allocation count and total bytes requested by **this thread** inside `f`.
fn measure<F: FnOnce()>(f: F) -> (usize, usize) {
    ALLOCS.with(|a| a.set(0));
    BYTES.with(|b| b.set(0));
    COUNTING.with(|c| c.set(true));
    f();
    COUNTING.with(|c| c.set(false));
    (ALLOCS.with(|a| a.get()), BYTES.with(|b| b.get()))
}

/// Warms the system up, then measures **two consecutive** frames and returns the worse one.
///
/// Two details this had to learn the hard way. **One warm-up frame is not enough**: a system can
/// reach steady state on its third frame, not its second — `HierarchySystem` adds
/// `GlobalTransform` on frame 1 (archetype migration) and only creates its change-tracking entry
/// on frame 2, so frame 2 measures 207 allocations and frame 3 measures none. A single warm-up
/// reported that transient as a per-frame leak. **And measuring one frame is not enough**: a
/// system that allocates every *other* frame would pass half the time. Two consecutive frames,
/// worst one wins.
fn steady_state_frame(system: &mut dyn System, world: &mut World, dt: f32) -> (usize, usize) {
    for _ in 0..WARMUP_FRAMES {
        system.run(world, dt);
    }
    let first = measure(|| system.run(world, dt));
    let second = measure(|| system.run(world, dt));
    if first.0 >= second.0 {
        first
    } else {
        second
    }
}

/// Frames a system is allowed before it must be at steady state. Three, because reaching it can
/// take two (see [`steady_state_frame`]); the third is slack, not a licence to be slow to settle.
const WARMUP_FRAMES: usize = 3;

/// The assertion every test here makes, with a message that names the system and the numbers.
fn assert_no_steady_state_allocation(name: &str, (allocs, bytes): (usize, usize)) {
    assert_eq!(
        allocs, 0,
        "{name} allocated {allocs} times ({bytes} bytes) on an unchanged steady-state frame — \
         a system that runs every frame must keep its temporaries as scratch fields"
    );
}

// ── Fixtures ──────────────────────────────────────────────────────────────────────────────────

/// A world with `n` particles that are nowhere near expiring, so the frame under measurement
/// neither despawns nor emits — the pure "update what exists" path a running game spends its
/// frames in.
fn particle_world(n: usize) -> World {
    let mut world = World::new();
    for i in 0..n {
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: Vec2::new(i as f32, i as f32),
                ..Default::default()
            },
        );
        world.add_component(e, Sprite::default());
        world.add_component(
            e,
            Particle {
                // Long lifetime and a fresh age: nothing expires during the test.
                age: 0.0,
                lifetime: 1_000.0,
                velocity: Vec2::new(1.0, 0.0),
                gravity: Vec2::ZERO,
                color_start: Color::rgba(1.0, 1.0, 1.0, 1.0),
                color_end: Color::rgba(1.0, 1.0, 1.0, 0.0),
            },
        );
    }
    world
}

fn collider_world(n: usize) -> World {
    let mut world = World::new();
    for i in 0..n {
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: Vec2::new((i % 40) as f32 * 50.0, (i / 40) as f32 * 50.0),
                ..Default::default()
            },
        );
        world.add_component(e, Collider::Circle { radius: 16.0 });
        world.add_component(e, CollisionLayer::ALL);
    }
    world
}

// ── The measurements ──────────────────────────────────────────────────────────────────────────

/// `ParticleSystem` must not allocate **per particle** — one bulk buffer is acceptable, 500 is
/// not.
///
/// This is deliberately a weaker assertion than the others, and the reason is worth stating so
/// nobody "tightens" it without reading this. §10 proposed replacing the collect with
/// `query3_mut`, which would take this to zero. It would also silently change behaviour: the
/// current code advances `age` through `get_mut::<Particle>` regardless of what else the entity
/// carries, while `query3_mut::<Transform, Sprite, Particle>` skips any particle missing one of
/// the three. The engine's own spawner always attaches all three, but a hand-spawned `Particle`
/// would stop ageing — and a particle that never ages never reaches its lifetime, so it would
/// never despawn. Trading one bulk allocation for a leak that only appears in forked game code
/// is a bad trade, and `ParticleSystem` is a unit struct constructed bare in seven places here
/// alone, so the scratch-field route is a breaking change for one allocation.
///
/// Measured: 1 allocation / 32,000 bytes for 500 particles — a single `Vec<ParticleUpdate>`.
#[test]
fn particle_system_does_not_allocate_per_particle() {
    let mut world = particle_world(500);
    let mut system = ParticleSystem;
    let (allocs, bytes) = steady_state_frame(&mut system, &mut world, 1.0 / 60.0);
    assert!(
        allocs <= 1,
        "ParticleSystem allocated {allocs} times ({bytes} bytes) for 500 particles — at most one \
         bulk buffer is allowed; anything more means an allocation crept into the per-particle loop"
    );
}

/// `SpatialGrid::rebuild` clears a `HashMap<_, Vec<Entity>>` every frame. `HashMap::clear` DROPS
/// the values, so every bucket's `Vec` is freed and reallocated on the next rebuild — the
/// capacity a steady scene would otherwise reuse is thrown away 60 times a second.
#[test]
fn spatial_grid_rebuild_steady_state_does_not_allocate() {
    let world = collider_world(400);
    let mut grid = SpatialGrid::new(128.0);
    grid.rebuild(&world);
    let measured = measure(|| grid.rebuild(&world));
    assert_no_steady_state_allocation("SpatialGrid::rebuild", measured);
}

/// `LocalizationSystem` was half-fixed in v0.150.0: the *write* side stops cloning when the text
/// is already correct, but the read side still builds two `Vec`s and two `String`s per entity
/// every frame before discovering there is nothing to do.
#[test]
fn localization_system_steady_state_does_not_allocate() {
    let mut world = World::new();
    world.insert_resource(LocaleResource::default());
    for _ in 0..200 {
        let e = world.spawn();
        world.add_component(e, LocalizedText::new("menu.start"));
        world.add_component(e, Label::new(""));
    }
    let mut system = LocalizationSystem;
    // Two warm-up frames: the first resolves the text, the second proves it is already correct,
    // so the measured third frame is pure steady state.
    system.run(&mut world, 0.0);
    let measured = steady_state_frame(&mut system, &mut world, 0.0);
    assert_no_steady_state_allocation("LocalizationSystem", measured);
}

/// The harness itself must be able to see an allocation, or every test above passes vacuously.
#[test]
fn the_counter_actually_counts() {
    let (allocs, bytes) = measure(|| {
        let v: Vec<u64> = Vec::with_capacity(128);
        std::hint::black_box(&v);
    });
    assert!(
        allocs >= 1 && bytes >= 1024,
        "the counting allocator saw {allocs} allocs / {bytes} bytes for a 1 KiB Vec — \
         if this reads zero, every other test in this file is vacuous"
    );
}

/// And it must read zero for work that genuinely does not allocate, or the tests above would be
/// impossible to satisfy and someone would "fix" them by loosening the assertion.
#[test]
fn the_counter_reads_zero_for_allocation_free_work() {
    let mut buf: Vec<u64> = Vec::with_capacity(256);
    let (allocs, _) = measure(|| {
        buf.clear();
        for i in 0..256u64 {
            buf.push(i);
        }
        std::hint::black_box(&buf);
    });
    assert_eq!(allocs, 0, "refilling a pre-sized Vec must not allocate");
}

// ── v0.150.0's claims, measured for the first time ────────────────────────────────────────────
//
// v0.150.0 fixed six of these on a code-reading basis and nothing has checked them since. A claim
// that a system stopped allocating deserves the same instrument as a claim that it allocates.

/// `HierarchySystem` is the one built-in **every game is forced to pay for** — `App` registers it
/// unconditionally and it runs over every entity with a `Transform` whether or not the game uses
/// hierarchies. v0.150.0 turned its six per-frame containers into scratch buffers; this is the
/// first thing to confirm that actually happened.
#[test]
fn hierarchy_system_steady_state_does_not_allocate() {
    let mut world = World::new();
    // A parented chain plus loose entities: exercises the topological sort, not just the trivial
    // "no Parent anywhere" path.
    let mut previous = None;
    for i in 0..200 {
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: Vec2::new(i as f32, 0.0),
                ..Default::default()
            },
        );
        if i % 2 == 1 {
            if let Some(parent) = previous {
                world.add_component(e, Parent(parent));
            }
        }
        previous = Some(e);
    }
    let mut system = HierarchySystem::default();
    let measured = steady_state_frame(&mut system, &mut world, 1.0 / 60.0);
    assert_no_steady_state_allocation("HierarchySystem", measured);
}

/// `TilemapSystem` deep-cloned the whole tile grid *before* its own "nothing changed" fast path —
/// v0.150.0 called it the single biggest per-frame allocation in the engine and moved the cheap
/// checks first. An idle tilemap must now copy nothing.
#[test]
fn tilemap_system_steady_state_does_not_allocate() {
    let mut world = World::new();
    let mut system = TilemapSystem::new();
    let measured = steady_state_frame(&mut system, &mut world, 1.0 / 60.0);
    assert_no_steady_state_allocation("TilemapSystem (idle)", measured);
}

// ── The leftovers v0.150.0 named and nobody wrote down ────────────────────────────────────────

/// `LayoutSystem` snapshots every panel each frame, cloning each panel's child list. Named as
/// "not addressed" by v0.150.0 and then recorded nowhere the backlog looks, so it survived three
/// more releases unmeasured.
#[test]
fn layout_system_steady_state_does_not_allocate() {
    let mut world = World::new();
    for _ in 0..50 {
        let panel = world.spawn();
        world.add_component(panel, UiNode::new(0.0, 0.0, 200.0, 400.0));
        let mut p = Panel::new(LayoutDir::Vertical);
        for _ in 0..8 {
            let child = world.spawn();
            world.add_component(child, UiNode::new(0.0, 0.0, 180.0, 40.0));
            p.children.push(child);
        }
        world.add_component(panel, p);
    }
    let mut system = LayoutSystem;
    let measured = steady_state_frame(&mut system, &mut world, 1.0 / 60.0);
    assert_no_steady_state_allocation("LayoutSystem", measured);
}

/// The **real** per-frame sequence, which the system-only tests above do not cover.
///
/// `App` calls `World::clear_change_tracking()` at the start of every frame. That used to
/// `HashMap::clear` the per-entity sets — dropping them — so every entity that changed last frame
/// paid a fresh `HashSet` to be recorded again this frame. `HierarchySystem` writes
/// `GlobalTransform` for **every** entity with a `Transform`, so that was a per-entity allocation
/// per frame in the one built-in every game is forced to register, and no system-only test could
/// see it: without the `clear`, the entry survives and the cost vanishes.
///
/// Measured before the fix: 200 allocations per frame for 200 entities, on top of the 200
/// `Box<GlobalTransform>` the write itself was doing.
#[test]
fn a_full_frame_including_change_tracking_reset_does_not_allocate() {
    let mut world = World::new();
    for i in 0..200 {
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: Vec2::new(i as f32, 0.0),
                ..Default::default()
            },
        );
    }
    let mut system = HierarchySystem::default();

    // A frame is: reset tracking, then run the systems.
    let frame = |world: &mut World, system: &mut HierarchySystem| {
        world.clear_change_tracking();
        system.run(world, 1.0 / 60.0);
    };
    for _ in 0..WARMUP_FRAMES {
        frame(&mut world, &mut system);
    }
    let first = measure(|| frame(&mut world, &mut system));
    let second = measure(|| frame(&mut world, &mut system));
    let worst = if first.0 >= second.0 { first } else { second };
    assert_no_steady_state_allocation("clear_change_tracking + HierarchySystem", worst);
}
