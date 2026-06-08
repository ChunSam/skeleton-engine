//! 2D cutout skeletal animation demo.
//!
//! Builds a humanoid puppet from colored rectangles only (no art assets required).
//! Includes a depth-5 bone chain (hip→torso→upper_arm→forearm→hand) to exercise
//! arbitrary-depth propagation in `HierarchySystem`.
//!
//! Controls: Space = toggle idle ↔ wave, Esc = quit.
//!
//! Run: `cargo run --example skeletal_puppet`

use engine::{
    App, BoneKeyframe, BoneTrack, InputState, KeyCode, ShouldQuit, SkeletalAnimationSystem,
    SkeletalAnimator, SkeletalClip, SkeletonBuilder, Sprite, System, Transform, Vec2, WindowConfig,
    World,
};

/// Attaches a visual rectangle as a child of a joint bone (scale=1, no sprite).
///
/// Keeping the joint scale at 1 prevents scale multiplication from exploding during
/// hierarchy composition. The visual child is a leaf and carries only its own size (scale).
fn add_visual(
    builder: &mut SkeletonBuilder,
    world: &mut World,
    joint: &str,
    size: Vec2,
    offset: Vec2,
    color: [f32; 3],
) {
    builder.add_bone(
        world,
        format!("{joint}_visual"),
        joint,
        Transform {
            position: offset,
            scale: size,
            rotation: 0.0,
            z: 0.0,
        },
        Some(Sprite::colored(color[0], color[1], color[2])),
    );
}

/// Builds a single track for one joint (rotation-only keyframes).
fn rot_track(joint: &str, keys: &[(f32, f32)]) -> BoneTrack {
    BoneTrack {
        bone: joint.to_string(),
        keys: keys
            .iter()
            .map(|&(time, rot)| BoneKeyframe {
                time,
                position: Vec2::ZERO,
                rotation: rot,
                scale: Vec2::ONE,
            })
            .collect(),
    }
}

/// Space toggles idle ↔ wave; Esc quits.
struct ControlSystem;

impl System for ControlSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (toggle, quit) = match world.resource::<InputState>() {
            Some(i) => (
                i.just_pressed(KeyCode::Space),
                i.just_pressed(KeyCode::Escape),
            ),
            None => (false, false),
        };
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.0 = true;
            }
        }
        if toggle {
            let roots: Vec<_> = world.query::<SkeletalAnimator>().map(|(e, _)| e).collect();
            for e in roots {
                if let Some(anim) = world.get_mut::<SkeletalAnimator>(e) {
                    let next = if anim.current == 0 { "wave" } else { "idle" };
                    anim.play_named(next);
                }
            }
        }
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeletal puppet — Space: idle/wave, Esc: quit".to_string(),
        width: 960,
        height: 540,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });

    // ── Joint bones (scale=1, position/rotation only) ───────────────────────────
    let joint = |x: f32, y: f32| Transform {
        position: Vec2::new(x, y),
        scale: Vec2::ONE,
        rotation: 0.0,
        z: 0.0,
    };

    let mut b = SkeletonBuilder::new(&mut app.world, "hip", joint(480.0, 200.0));
    b.add_bone(&mut app.world, "torso", "hip", joint(0.0, 30.0), None);
    b.add_bone(&mut app.world, "head", "torso", joint(0.0, 95.0), None);
    // Right arm chain: depth hip→torso→r_upper_arm→r_forearm→r_hand (5)
    b.add_bone(
        &mut app.world,
        "r_upper_arm",
        "torso",
        joint(35.0, 75.0),
        None,
    );
    b.add_bone(
        &mut app.world,
        "r_forearm",
        "r_upper_arm",
        joint(0.0, -45.0),
        None,
    );
    b.add_bone(
        &mut app.world,
        "r_hand",
        "r_forearm",
        joint(0.0, -40.0),
        None,
    );
    // Left arm
    b.add_bone(
        &mut app.world,
        "l_upper_arm",
        "torso",
        joint(-35.0, 75.0),
        None,
    );
    b.add_bone(
        &mut app.world,
        "l_forearm",
        "l_upper_arm",
        joint(0.0, -45.0),
        None,
    );
    // Legs
    b.add_bone(&mut app.world, "l_leg", "hip", joint(-18.0, -10.0), None);
    b.add_bone(&mut app.world, "r_leg", "hip", joint(18.0, -10.0), None);

    // ── Visual rectangles ────────────────────────────────────────────────────────
    let skin = [0.90, 0.78, 0.65];
    let shirt = [0.30, 0.55, 0.85];
    let pants = [0.25, 0.28, 0.35];
    add_visual(
        &mut b,
        &mut app.world,
        "torso",
        Vec2::new(50.0, 80.0),
        Vec2::new(0.0, 30.0),
        shirt,
    );
    add_visual(
        &mut b,
        &mut app.world,
        "head",
        Vec2::new(44.0, 44.0),
        Vec2::ZERO,
        skin,
    );
    add_visual(
        &mut b,
        &mut app.world,
        "r_upper_arm",
        Vec2::new(16.0, 45.0),
        Vec2::new(0.0, -22.0),
        shirt,
    );
    add_visual(
        &mut b,
        &mut app.world,
        "r_forearm",
        Vec2::new(14.0, 40.0),
        Vec2::new(0.0, -20.0),
        skin,
    );
    add_visual(
        &mut b,
        &mut app.world,
        "r_hand",
        Vec2::new(16.0, 16.0),
        Vec2::ZERO,
        skin,
    );
    add_visual(
        &mut b,
        &mut app.world,
        "l_upper_arm",
        Vec2::new(16.0, 45.0),
        Vec2::new(0.0, -22.0),
        shirt,
    );
    add_visual(
        &mut b,
        &mut app.world,
        "l_forearm",
        Vec2::new(14.0, 40.0),
        Vec2::new(0.0, -20.0),
        skin,
    );
    add_visual(
        &mut b,
        &mut app.world,
        "l_leg",
        Vec2::new(18.0, 60.0),
        Vec2::new(0.0, -30.0),
        pants,
    );
    add_visual(
        &mut b,
        &mut app.world,
        "r_leg",
        Vec2::new(18.0, 60.0),
        Vec2::new(0.0, -30.0),
        pants,
    );

    // ── Clips ────────────────────────────────────────────────────────────────────
    // idle: torso sways gently side-to-side, arms sway slightly (looping)
    let idle = SkeletalClip {
        name: "idle".into(),
        duration: 2.0,
        looping: true,
        tracks: vec![
            rot_track("torso", &[(0.0, -0.04), (1.0, 0.04), (2.0, -0.04)]),
            rot_track("r_upper_arm", &[(0.0, 0.05), (1.0, -0.05), (2.0, 0.05)]),
            rot_track("l_upper_arm", &[(0.0, -0.05), (1.0, 0.05), (2.0, -0.05)]),
        ],
    };
    // wave: raises right arm and waves the forearm side-to-side (looping)
    let wave = SkeletalClip {
        name: "wave".into(),
        duration: 1.0,
        looping: true,
        tracks: vec![
            // Raise the right upper arm (~-2.6 rad ≈ beside the head)
            rot_track("r_upper_arm", &[(0.0, -2.6), (1.0, -2.6)]),
            // Wave the forearm side-to-side
            rot_track("r_forearm", &[(0.0, -0.5), (0.5, 0.5), (1.0, -0.5)]),
            rot_track("torso", &[(0.0, 0.0), (0.5, 0.03), (1.0, 0.0)]),
        ],
    };

    b.finish(&mut app.world, vec![idle, wave]);

    app.add_system(SkeletalAnimationSystem);
    app.add_system(ControlSystem);
    app.run();
}
