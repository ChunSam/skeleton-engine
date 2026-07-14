//! Where the engine looks for a relative asset path, and what it does when it can't find one.
//!
//! # The problem this solves
//!
//! Every asset API in the engine is path-based (`App::load_image("assets/hero.png")`,
//! `AudioManager::play("sfx", "assets/hit.wav", false)`, `App::load_data_table(…)`), and the
//! path doubles as the asset's identity (its cache key / handle path). Reading it, though, used
//! to go straight to `std::fs`, which resolves a *relative* path against the **process working
//! directory**.
//!
//! That means a shipped executable only worked when launched from one specific directory.
//! Double-clicked in a file manager, started from a desktop shortcut, or run as
//! `cd /elsewhere && game.exe`, every texture load failed, the renderer substituted its magenta
//! fallback for each one, and — because a title screen is usually one full-screen textured quad
//! — the whole window turned magenta with nothing but a `warn!` to say why.
//!
//! # What happens instead
//!
//! [`resolve`] turns a relative asset path into an absolute one against an **asset root the
//! engine determines**, not the working directory. With no configuration, it searches, in order:
//!
//! 1. a macOS app bundle's `Contents/Resources` (when the executable sits in `Contents/MacOS`),
//! 2. the executable's own directory, then its ancestors (up to [`ANCESTOR_DEPTH`]),
//! 3. the working directory, then its ancestors.
//!
//! The first candidate under which the file **actually exists** wins. A packaged build therefore
//! finds the `assets/` sitting next to its executable no matter how it was launched, while a dev
//! build run from a repo checkout (`cargo run --example …`, where the executable lives in
//! `target/debug/examples/`) still resolves against the repo root — which is why adopting this
//! changed no existing example.
//!
//! Executable-derived candidates are searched **before** the working directory so that a shipped
//! build never silently picks up a stray `assets/` from wherever the user happened to launch it.
//!
//! [`set_asset_root`] overrides the search with a single explicit root — useful for a dev build,
//! a test, or a game that stores its content somewhere unusual.
//!
//! # One failure list, for every kind of asset
//!
//! Resolution is only half the fix. A load that still fails — a missing file, a RON syntax error —
//! must be *noticeable*, and the fallback each subsystem picks is easy to miss: a magenta texture,
//! silence, an **empty data table**. A downstream game shipped a packaged build whose 19 data
//! tables all resolved to nothing; every table registered empty, so the game booted "correctly"
//! with an empty shop and no dungeons, and it took a player's bug report to find. A silent empty
//! table is indistinguishable from a legitimately empty one.
//!
//! So every path-based loader — textures, images, data tables, animation clip sets, particle
//! configs, dialogue trees, trigger-zone sets, zone- and anim-effect tables, Rhai scripts, audio
//! clips — reports its failures the same way: logged at `error!` **with the roots it searched**,
//! and appended to [`asset_failures`]. That list is the hook: assert on it at boot and refuse to
//! start, or draw it in a diagnostic overlay. [`set_strict_assets`] turns any such failure into a
//! panic at the load instead.
//!
//! Two things are deliberately **not** failures here: a *hot-reload* that fails (the file loaded
//! once, and a half-saved edit should not kill a strict dev build — those still `warn!`), and an
//! intentionally empty asset that parses fine.
//!
//! # What it deliberately does not touch
//!
//! Resolution happens **only at the point of reading the file**. An asset's cache key and its
//! `Handle::path()` stay exactly the string the caller passed. Rewriting those to the resolved
//! path would reintroduce a bug this engine has already shipped once: a handle keyed by the
//! canonical path while the GPU texture cache is keyed by the relative one, so every sprite
//! silently renders white. Keep identity logical; keep resolution at the filesystem edge.
//!
//! Absolute paths are returned unchanged. On wasm there is no filesystem — paths are URLs — so
//! [`resolve`] is the identity function there.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};

/// How far to walk up from the executable / working directory looking for an asset root.
///
/// A dev build's executable sits at `<root>/target/debug/examples/<name>`, which is 3 levels
/// below the repo root; 4 leaves a little headroom.
pub const ANCESTOR_DEPTH: usize = 4;

/// An explicit root set by [`set_asset_root`]. `None` = search the default candidates.
static EXPLICIT_ROOT: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Assets the engine tried and failed to load, in load order. See [`asset_failures`].
static FAILURES: Mutex<Vec<AssetFailure>> = Mutex::new(Vec::new());

/// When true, a failed asset load panics instead of falling back. See [`set_strict_assets`].
static STRICT: RwLock<bool> = RwLock::new(false);

/// An asset the engine could not load. Retrieve with [`asset_failures`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetFailure {
    /// The path as the caller wrote it (not the resolved one).
    pub path: String,
    /// Why it failed — a missing file, a decode error, and so on.
    pub error: String,
}

/// Pins asset resolution to one explicit root directory, replacing the default search.
///
/// Relative asset paths are then resolved against `root` and nowhere else, so a miss is a miss
/// rather than quietly falling back to the working directory. Pass an absolute path unless you
/// genuinely want it interpreted relative to the process working directory.
///
/// ```rust,no_run
/// # use engine::asset_path;
/// // Dev build running from a checkout whose content lives outside the repo:
/// asset_path::set_asset_root("/opt/shared-content");
/// ```
pub fn set_asset_root(root: impl Into<PathBuf>) {
    let root = root.into();
    log::info!("asset root pinned to {}", root.display());
    *EXPLICIT_ROOT.write().unwrap() = Some(root);
}

/// The explicit root set by [`set_asset_root`], if any. `None` means the default search is used.
pub fn asset_root() -> Option<PathBuf> {
    EXPLICIT_ROOT.read().unwrap().clone()
}

/// Clears an explicit root, restoring the default candidate search.
pub fn clear_asset_root() {
    *EXPLICIT_ROOT.write().unwrap() = None;
}

/// Makes a failed asset load **panic** instead of falling back to a placeholder.
///
/// Off by default, because a released game should degrade rather than die. Turn it on in a dev
/// build and a missing texture stops the program at the load instead of painting the window
/// magenta and leaving you to guess.
pub fn set_strict_assets(strict: bool) {
    *STRICT.write().unwrap() = strict;
}

/// Whether strict mode is on. See [`set_strict_assets`].
pub fn strict_assets() -> bool {
    *STRICT.read().unwrap()
}

/// Every asset the engine failed to load so far, in load order.
///
/// The engine's own failure handling is a fallback (a magenta texture, silence), which keeps a
/// game running but is easy to miss. This is the hook for noticing: log it, show a diagnostic,
/// or refuse to start.
///
/// ```rust,no_run
/// # use engine::asset_path;
/// for f in asset_path::asset_failures() {
///     eprintln!("asset failed: {} — {}", f.path, f.error);
/// }
/// ```
pub fn asset_failures() -> Vec<AssetFailure> {
    FAILURES.lock().unwrap().clone()
}

/// Drops the recorded failures (e.g. after reporting them, or between scenes).
pub fn clear_asset_failures() {
    FAILURES.lock().unwrap().clear();
}

/// Records a failed load — and panics first when [`set_strict_assets`] is on.
///
/// The message carries the roots that were searched, since "file not found" is not much help when
/// the whole question is *where the engine looked*. Recorded once per path: one `App::load_image`
/// feeds both the `AssetServer` and the renderer's upload queue, so a missing file surfaces from
/// two subsystems and would otherwise be listed twice.
pub(crate) fn record_failure(path: &str, error: impl std::fmt::Display) {
    let roots = candidate_roots()
        .iter()
        .map(|r| r.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let error = if roots.is_empty() {
        error.to_string()
    } else {
        format!("{error} (searched: {roots})")
    };

    log::error!("asset load failed '{path}': {error}");
    if strict_assets() {
        panic!("asset load failed '{path}': {error} (strict asset mode is on)");
    }

    let mut failures = FAILURES.lock().unwrap();
    if failures.iter().any(|f| f.path == path) {
        return;
    }
    failures.push(AssetFailure {
        path: path.to_string(),
        error,
    });
}

/// `Foo.app/Contents/MacOS/foo` → `Foo.app/Contents/Resources`, where a macOS bundle keeps its
/// content. Returns `None` for an executable that is not inside a bundle.
#[cfg(not(target_arch = "wasm32"))]
fn macos_bundle_resources(exe: &Path) -> Option<PathBuf> {
    let macos_dir = exe.parent()?;
    if macos_dir.file_name()? != "MacOS" {
        return None;
    }
    let contents_dir = macos_dir.parent()?;
    if contents_dir.file_name()? != "Contents" {
        return None;
    }
    Some(contents_dir.join("Resources"))
}

#[cfg(not(target_arch = "wasm32"))]
fn push_unique(out: &mut Vec<PathBuf>, dir: PathBuf) {
    if !out.contains(&dir) {
        out.push(dir);
    }
}

/// The directories that may hold assets, in search order. Pure path arithmetic — the filesystem
/// is only consulted by [`resolve`], which is what makes this testable without a real layout.
///
/// An explicit root short-circuits the search: set one and it is the only candidate.
#[cfg(not(target_arch = "wasm32"))]
fn candidates_from(
    explicit: Option<&Path>,
    exe: Option<&Path>,
    cwd: Option<&Path>,
) -> Vec<PathBuf> {
    if let Some(root) = explicit {
        return vec![root.to_path_buf()];
    }

    let mut out = Vec::new();

    if let Some(exe) = exe {
        if let Some(resources) = macos_bundle_resources(exe) {
            push_unique(&mut out, resources);
        }
        if let Some(exe_dir) = exe.parent() {
            for ancestor in exe_dir.ancestors().take(ANCESTOR_DEPTH) {
                push_unique(&mut out, ancestor.to_path_buf());
            }
        }
    }

    if let Some(cwd) = cwd {
        for ancestor in cwd.ancestors().take(ANCESTOR_DEPTH) {
            push_unique(&mut out, ancestor.to_path_buf());
        }
    }

    out
}

/// The directories the engine will search for a relative asset path, in order.
///
/// Useful in a diagnostic: when an asset is missing, this is where the engine looked.
#[cfg(not(target_arch = "wasm32"))]
pub fn candidate_roots() -> Vec<PathBuf> {
    let explicit = asset_root();
    let exe = std::env::current_exe().ok();
    let cwd = std::env::current_dir().ok();
    candidates_from(explicit.as_deref(), exe.as_deref(), cwd.as_deref())
}

/// The resolution itself, against an explicit list of roots.
///
/// Split out from [`resolve`] so it can be tested against a temporary directory **without
/// touching the process-global root** — which a parallel test suite shares, and which a test that
/// mutated it would silently impose on every other test's asset loads.
#[cfg(not(target_arch = "wasm32"))]
fn resolve_in(roots: &[PathBuf], path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    for root in roots {
        let candidate = root.join(path);
        if candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

/// Resolves a relative asset path against the engine's asset root; see the [module docs](self).
///
/// An absolute path is returned unchanged. A relative path is returned joined to the first
/// candidate root under which it exists. If it exists under none of them, the path is returned
/// **unchanged**, so the ensuing error names the path the caller actually asked for.
#[cfg(not(target_arch = "wasm32"))]
pub fn resolve(path: impl AsRef<Path>) -> PathBuf {
    resolve_in(&candidate_roots(), path.as_ref())
}

/// wasm has no filesystem — asset paths are URLs fetched by the host — so resolution is identity.
#[cfg(target_arch = "wasm32")]
pub fn resolve(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().to_path_buf()
}

/// wasm has no filesystem, so there are no roots to search. Present on both targets so that
/// callers in cross-target modules (the renderer's texture loader) need no `cfg`.
#[cfg(target_arch = "wasm32")]
pub fn candidate_roots() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn a_macos_bundle_resources_dir_is_searched_first() {
        let exe = PathBuf::from("/Applications/Game.app/Contents/MacOS/game");
        let candidates = candidates_from(None, Some(&exe), None);

        assert_eq!(
            candidates.first().map(PathBuf::as_path),
            Some(Path::new("/Applications/Game.app/Contents/Resources"))
        );
    }

    #[test]
    fn a_plain_executable_yields_no_bundle_candidate() {
        let exe = PathBuf::from("/opt/game/bin/game");
        let candidates = candidates_from(None, Some(&exe), None);

        assert_eq!(
            candidates.first().map(PathBuf::as_path),
            Some(Path::new("/opt/game/bin"))
        );
    }

    #[test]
    fn executable_ancestors_reach_the_repo_root_of_a_dev_build() {
        // `cargo run --example foo` puts the binary three levels below the repo root — the dev
        // build must still find `examples/assets/…`, which is what keeps this change additive.
        let exe = PathBuf::from("/work/engine/target/debug/examples/foo");
        let candidates = candidates_from(None, Some(&exe), None);

        assert!(candidates.contains(&PathBuf::from("/work/engine")));
    }

    #[test]
    fn executable_candidates_precede_working_directory_candidates() {
        // A shipped build must not pick up a stray `assets/` from wherever it was launched.
        let exe = PathBuf::from("/opt/game/bin/game");
        let cwd = PathBuf::from("/home/player");
        let candidates = candidates_from(None, Some(&exe), Some(&cwd));

        let exe_pos = candidates
            .iter()
            .position(|d| d == Path::new("/opt/game/bin"))
            .expect("exe dir is a candidate");
        let cwd_pos = candidates
            .iter()
            .position(|d| d == Path::new("/home/player"))
            .expect("cwd is a candidate");

        assert!(exe_pos < cwd_pos);
    }

    #[test]
    fn candidates_are_deduplicated_when_exe_dir_and_cwd_overlap() {
        let exe = PathBuf::from("/work/game/game");
        let cwd = PathBuf::from("/work/game");
        let candidates = candidates_from(None, Some(&exe), Some(&cwd));

        let hits = candidates
            .iter()
            .filter(|d| *d == Path::new("/work/game"))
            .count();
        assert_eq!(hits, 1);
    }

    #[test]
    fn an_explicit_root_is_the_only_candidate() {
        let explicit = PathBuf::from("/opt/content");
        let exe = PathBuf::from("/opt/game/bin/game");
        let cwd = PathBuf::from("/home/player");
        let candidates = candidates_from(Some(&explicit), Some(&exe), Some(&cwd));

        assert_eq!(candidates, vec![explicit]);
    }

    #[test]
    fn candidates_are_empty_without_an_executable_or_working_directory() {
        assert!(candidates_from(None, None, None).is_empty());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn an_absolute_path_resolves_to_itself() {
        let abs = std::env::temp_dir().join("engine-asset-path-absolute.png");
        assert_eq!(resolve(&abs), abs);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_relative_path_that_exists_nowhere_is_returned_unchanged() {
        // Unchanged, so the ensuing error names the path the caller asked for.
        let missing = Path::new("__engine_no_such_asset__/nope.png");
        assert_eq!(resolve(missing), missing.to_path_buf());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_relative_path_resolves_against_a_root_that_is_not_the_working_directory() {
        // The whole point of the module: resolution must not depend on the working directory.
        //
        // Driven through `resolve_in` rather than `set_asset_root` + `resolve`, because the root
        // is process-global: a test that pinned it would impose that root on every other test's
        // asset loads running in parallel, breaking them at a distance.
        let dir = std::env::temp_dir().join(format!(
            "engine-asset-root-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let nested = dir.join("assets");
        std::fs::create_dir_all(&nested).unwrap();
        let file = nested.join("pixel.png");
        std::fs::write(&file, b"not really a png").unwrap();

        let resolved = resolve_in(std::slice::from_ref(&dir), Path::new("assets/pixel.png"));

        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(resolved, file);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_first_root_that_holds_the_file_wins_not_merely_the_first_root() {
        let dir = std::env::temp_dir().join(format!(
            "engine-asset-order-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let empty = dir.join("empty");
        let holds = dir.join("holds");
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::create_dir_all(&holds).unwrap();
        let file = holds.join("pixel.png");
        std::fs::write(&file, b"x").unwrap();

        let resolved = resolve_in(&[empty, holds], Path::new("pixel.png"));

        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(resolved, file);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_failure_is_recorded_once_per_path_and_can_be_cleared() {
        // The failure list is process-global and other tests in this binary record into it
        // concurrently, so assert on THIS path only — never on the list's length.
        let path = "assets/__unit_test_recorded_failure__.png";

        record_failure(path, "no such file");
        record_failure(path, "no such file"); // one `load_image` reports from two subsystems

        let mine: Vec<_> = asset_failures()
            .into_iter()
            .filter(|f| f.path == path)
            .collect();
        assert_eq!(mine.len(), 1, "a path must be recorded at most once");
        assert!(mine[0].error.contains("no such file"));
        assert!(
            mine[0].error.contains("searched:"),
            "the message must name the roots searched, or it doesn't help anyone: {}",
            mine[0].error
        );

        clear_asset_failures();
        assert!(
            !asset_failures().iter().any(|f| f.path == path),
            "clear_asset_failures must drop recorded failures"
        );
    }
}
