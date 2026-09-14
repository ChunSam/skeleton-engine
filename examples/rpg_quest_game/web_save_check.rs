//! Browser-only acceptance checks over the game's real save functions and quest schema.
//! `web/check.html` reloads between write, restore and migration: no World or wasm memory survives.
//! This deliberately starts no renderer; storage and quest reconstruction need neither a GPU nor
//! the game's file watcher (which has a separate native selftest).

use super::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn web_save_key() -> String {
    save_file().to_string_lossy().into_owned()
}

fn require(ok: bool, message: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn expected_quest() -> QuestState {
    QuestState {
        gold: 37,
        mine_cleared: true,
        town_enters: 4,
        lantern_name: "달빛 lantern 🏮".into(),
        has_lantern: true,
        knows_mine: true,
    }
}

fn missing_save() -> bool {
    matches!(read_save(), Err(save::SaveError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound)
}

fn check(phase: &str) -> Result<(), String> {
    match phase {
        "write" => {
            // Refuse to overwrite a pre-existing save if a human opens this page in their browser.
            require(!save::exists(&save_file()), "fixture save already exists")?;
            require(missing_save(), "absent save did not return NotFound")?;
            let mut world = World::new();
            let mut quest = expected_quest();
            // DialogueVars is the authority during play. Saving must snapshot it, not these
            // deliberately stale flags; the next page checks the complete expected quest.
            quest.has_lantern = false;
            quest.knows_mine = false;
            world.insert_resource(quest);
            let mut vars = DialogueVars::new();
            vars.set_bool(VAR_LANTERN, true);
            vars.set_bool(VAR_KNOWS_MINE, true);
            vars.set_int(VAR_GOLD, expected_quest().gold);
            world.insert_resource(vars);
            write_save(&mut world).map_err(|e| format!("write v1: {e}"))?;
            require(save::exists(&save_file()), "write did not persist a save")
        }
        "restore" => {
            let loaded = read_save().map_err(|e| format!("read v1 after reload: {e}"))?;
            require(
                loaded.quest == expected_quest(),
                &format!("quest changed across reload: {:?}", loaded.quest),
            )?;
            // Same reconstruction as the game's load action, starting from a closed quest gate.
            let mut world = World::new();
            let vars = DialogueVars::new();
            require(!gate_open(&vars), "fresh quest gate was already open")?;
            world.insert_resource(vars);
            rebuild_vars(&mut world, &loaded.quest);
            world.insert_resource(loaded.quest);
            let vars = world
                .resource::<DialogueVars>()
                .ok_or("missing dialogue bag")?;
            require(
                gate_open(vars),
                "restored dialogue flags did not open the gate",
            )?;
            require(vars.get_int(VAR_GOLD) == Some(37), "restored gold changed")?;

            // Match the game's native v0 fixture: the migrator must supply lantern_name.
            #[derive(Serialize)]
            struct QuestV0 {
                gold: i64,
                mine_cleared: bool,
                town_enters: u32,
                has_lantern: bool,
                knows_mine: bool,
            }
            #[derive(Serialize)]
            struct SaveV0 {
                quest: QuestV0,
            }
            let old = SaveV0 {
                quest: QuestV0 {
                    gold: 3,
                    mine_cleared: true,
                    town_enters: 5,
                    has_lantern: true,
                    knows_mine: true,
                },
            };
            save::save_versioned(&save_file(), 0, &old).map_err(|e| format!("write v0: {e}"))
        }
        "migrate" => {
            let loaded = read_save().map_err(|e| format!("migrate v0 after reload: {e}"))?;
            let expected = QuestState {
                gold: 3,
                town_enters: 5,
                lantern_name: "brass lantern".into(),
                ..expected_quest()
            };
            require(
                loaded.quest == expected,
                &format!("migrated quest changed: {:?}", loaded.quest),
            )
        }
        "corrupted" => require(
            matches!(read_save(), Err(save::SaveError::Corrupted)),
            "tampered ciphertext did not return Corrupted",
        ),
        "delete" => {
            save::delete(&save_file()).map_err(|e| format!("delete: {e}"))?;
            require(!save::exists(&save_file()), "deleted save still exists")?;
            require(missing_save(), "deleted save did not return NotFound")
        }
        _ => Err(format!("unknown phase: {phase}")),
    }
}

#[wasm_bindgen]
pub fn web_check_save(phase: &str) -> Result<(), JsValue> {
    check(phase).map_err(|e| JsValue::from_str(&format!("{phase}: {e}")))
}
