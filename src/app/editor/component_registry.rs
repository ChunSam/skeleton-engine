#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::tr;
use crate::app::App;
use crate::ecs::{Entity, World};

#[cfg(not(target_arch = "wasm32"))]
use super::ui;

#[cfg(not(target_arch = "wasm32"))]
impl App {
    pub(in crate::app) fn register_default_components(&mut self) {
        self.register_component("Sprite", |world, e| {
            world.add_component(e, crate::components::Sprite::default());
        });
        self.register_component("RenderLayer", |world, e| {
            world.add_component(e, crate::components::RenderLayer::default());
        });
        self.register_component("SpriteFlip", |world, e| {
            world.add_component(e, crate::components::SpriteFlip::default());
        });
        self.register_component("ParticleEmitter", |world, e| {
            world.add_component(e, crate::particle::ParticleEmitter::default());
        });
        self.register_component("PointLight", |world, e| {
            world.add_component(e, crate::components::PointLight::default());
        });
        // UI widget component factories — names must match the serde registration names
        // used by `core_resources::init_ui_serde_components` so "+ Add Component" and
        // scene save/load agree on the key strings.
        self.register_component("UiNode", |world, e| {
            world.add_component(e, crate::ui::UiNode::default());
        });
        self.register_component("Button", |world, e| {
            world.add_component(e, crate::ui::Button::default());
        });
        self.register_component("Label", |world, e| {
            world.add_component(e, crate::ui::Label::default());
        });
        self.register_component("TextInput", |world, e| {
            world.add_component(e, crate::ui::TextInput::default());
        });
        self.register_component("Slider", |world, e| {
            world.add_component(e, crate::ui::Slider::default());
        });
        self.register_component("CheckBox", |world, e| {
            world.add_component(e, crate::ui::CheckBox::default());
        });
        self.register_component("ScrollView", |world, e| {
            world.add_component(e, crate::ui::ScrollView::default());
        });
        self.register_component("Panel", |world, e| {
            world.add_component(e, crate::ui::panel::Panel::default());
        });
        // register removal closures — the Inspector "✕" button uses this map to show/act
        self.register_component_remover("Sprite", |world, e| {
            world.remove_component::<crate::components::Sprite>(e);
        });
        self.register_component_remover("RenderLayer", |world, e| {
            world.remove_component::<crate::components::RenderLayer>(e);
        });
        self.register_component_remover("SpriteFlip", |world, e| {
            world.remove_component::<crate::components::SpriteFlip>(e);
        });
        self.register_component_remover("ParticleEmitter", |world, e| {
            world.remove_component::<crate::particle::ParticleEmitter>(e);
        });
        self.register_component_remover("PointLight", |world, e| {
            world.remove_component::<crate::components::PointLight>(e);
        });
        self.register_component_remover("Tag", |world, e| {
            world.remove_component::<crate::prefab::Tag>(e);
        });
        self.register_component_remover("UiNode", |world, e| {
            world.remove_component::<crate::ui::UiNode>(e);
        });
        self.register_component_remover("Button", |world, e| {
            world.remove_component::<crate::ui::Button>(e);
        });
        self.register_component_remover("Label", |world, e| {
            world.remove_component::<crate::ui::Label>(e);
        });
        self.register_component_remover("TextInput", |world, e| {
            world.remove_component::<crate::ui::TextInput>(e);
        });
        self.register_component_remover("Slider", |world, e| {
            world.remove_component::<crate::ui::Slider>(e);
        });
        self.register_component_remover("CheckBox", |world, e| {
            world.remove_component::<crate::ui::CheckBox>(e);
        });
        self.register_component_remover("ScrollView", |world, e| {
            world.remove_component::<crate::ui::ScrollView>(e);
        });
        self.register_component_remover("Panel", |world, e| {
            world.remove_component::<crate::ui::panel::Panel>(e);
        });

        // ── Built-in inspector sub-panels ─────────────────────────────────────
        // Registered via the same API available to forkers; order here determines
        // the render order in the docked inspector.
        self.register_inspector_panel::<crate::particle::ParticleEmitter>(
            tr("Particle Tuner", "파티클 튜너"),
            |ui_panel, app, e| {
                ui::particle_tuner_grid(ui_panel, app, e);
                if ui_panel
                    .button(tr("↺ Reset to Default", "↺ 기본값으로 초기화"))
                    .on_hover_text(tr(
                        "reset all fields (keeps the texture)",
                        "모든 필드 초기화 (텍스처 유지)",
                    ))
                    .clicked()
                {
                    app.reset_particle_emitter(e);
                }
                ui_panel.label(
                    egui::RichText::new(tr(
                        "edits apply live while the sim runs (unpause)",
                        "시뮬레이션 실행 중 실시간 적용 (일시정지 해제)",
                    ))
                    .weak(),
                );
            },
        );

        self.register_inspector_panel::<crate::components::PointLight>(
            tr("Point Light", "포인트 라이트"),
            |ui_panel, app, e| {
                ui::point_light_grid(ui_panel, app, e);
                if ui_panel
                    .button(tr("↺ Reset to Default", "↺ 기본값으로 초기화"))
                    .on_hover_text(tr(
                        "reset color / radius / intensity / height",
                        "색상 / 반경 / 강도 / 높이 초기화",
                    ))
                    .clicked()
                {
                    app.reset_point_light(e);
                }
                ui_panel.label(
                    egui::RichText::new(tr(
                        "the entity's Transform position is the light position",
                        "엔티티의 트랜스폼 위치가 빛의 위치입니다",
                    ))
                    .weak(),
                );
            },
        );

        self.register_inspector_panel::<crate::animation::AnimationStateMachine>(
            tr("State Machine", "상태 머신"),
            |ui_panel, app, e| {
                ui::state_machine_panel(ui_panel, app, e);
            },
        );

        self.register_inspector_panel::<crate::timeline::Timeline>(
            tr("Timeline", "타임라인"),
            |ui_panel, app, e| {
                ui::timeline_panel(ui_panel, app, e);
            },
        );
    }

    pub fn register_component(
        &mut self,
        name: impl Into<String>,
        factory: impl Fn(&mut World, Entity) + Send + Sync + 'static,
    ) {
        self.editor
            .component_factories
            .insert(name.into(), Box::new(factory));
    }

    /// Registers a removal closure so the Inspector can remove a component.
    /// Call this with the same name as `register_component` to make custom components
    /// removable; components absent from this map have their "✕" button hidden.
    pub fn register_component_remover(
        &mut self,
        name: impl Into<String>,
        remover: impl Fn(&mut World, Entity) + Send + Sync + 'static,
    ) {
        self.editor
            .component_removers
            .insert(name.into(), Box::new(remover));
    }
}

impl App {
    /// Registers a component for full editor integration in one call:
    /// Inspector field editing ([`Reflect`](crate::reflect::Reflect)), entity duplication
    /// ([`Clone`]), scene save/load (serde), and the Add/Remove Component buttons.
    ///
    /// `T` must derive `Reflect`, `Serialize`, `Deserialize`, `Clone`, and `Default`.
    ///
    /// This is the preferred registration path for game-side stats/config components.
    /// On wasm only the reflect + clone + serde registrations run (the editor buttons
    /// are native-only); the method still compiles on both targets.
    ///
    /// # Platform-gated registrations
    ///
    /// [`register_component`](Self::register_component) (the "Add Component" factory) and
    /// [`register_component_remover`](Self::register_component_remover) (the "Remove
    /// Component" button) are **native-only** — they drive the docked editor UI which does
    /// not exist on wasm. On wasm only the reflect, clone, and serde registrations apply,
    /// which still cover `Inspector` field display, entity clone, and scene round-trips.
    ///
    /// # Example
    /// ```rust,no_run
    /// use engine::App;
    /// use engine_reflect_derive::Reflect;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Reflect, Serialize, Deserialize, Clone, Default)]
    /// struct Stats { hp: f32, strength: i32 }
    ///
    /// let mut app = App::new();
    /// app.register_editable_component::<Stats>("Stats", None);
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn register_editable_component<T>(
        &mut self,
        name: &'static str,
        post_spawn: Option<Box<dyn Fn(&mut World, Entity) + Send + Sync>>,
    ) where
        T: crate::reflect::Reflect
            + serde::Serialize
            + serde::de::DeserializeOwned
            + Clone
            + Default
            + Send
            + Sync
            + 'static,
    {
        self.world.register_reflect_named::<T>(name);
        self.world.register_clone::<T>();
        // Push a replay thunk for reflect+clone so they survive scene Replace.
        // `name` is `&'static str` (Copy), so no clone needed.
        self.world_registrars.push(Box::new(move |world| {
            world.register_reflect_named::<T>(name);
            world.register_clone::<T>();
        }));
        self.register_serde_component::<T>(name, post_spawn);
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.register_component(name, |world, entity| {
                world.add_component(entity, T::default());
            });
            self.register_component_remover(name, |world, entity| {
                world.remove_component::<T>(entity);
            });
        }
    }

    /// Registers a custom inspector sub-panel that appears in the docked editor whenever
    /// the selected entity has component `T`.
    ///
    /// The panel is rendered as a [`egui::CollapsingHeader`] with the given `title`,
    /// open by default, inserted **after** the built-in sub-panels (Particle Tuner,
    /// Point Light, State Machine, Timeline) and **before** the Component List.
    ///
    /// This is a **native-only** method — it compiles out on wasm. Gate any call
    /// site with `#[cfg(not(target_arch = "wasm32"))]` when needed.
    ///
    /// # Example
    /// ```rust,no_run
    /// use engine::App;
    ///
    /// #[derive(Default)]
    /// struct HeatSource { temperature: f32, radius: f32 }
    ///
    /// let mut app = App::new();
    /// app.register_inspector_panel::<HeatSource>("Heat Source", |ui, app, entity| {
    ///     if let Some(h) = app.world.get_mut::<HeatSource>(entity) {
    ///         ui.horizontal(|ui| {
    ///             ui.label("temperature");
    ///             ui.add(egui::DragValue::new(&mut h.temperature).speed(0.5));
    ///         });
    ///         ui.horizontal(|ui| {
    ///             ui.label("radius");
    ///             ui.add(egui::DragValue::new(&mut h.radius).range(0.0..=1000.0).speed(1.0));
    ///         });
    ///     }
    /// });
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn register_inspector_panel<T: 'static>(
        &mut self,
        title: impl Into<String>,
        draw: impl Fn(&mut egui::Ui, &mut App, Entity) + 'static,
    ) {
        self.editor
            .inspector_panels
            .push(crate::app::editor::InspectorPanel {
                presence: |world, entity| world.has_component::<T>(entity),
                title: title.into(),
                draw: Box::new(draw),
            });
    }

    /// Registers a serde-capable component type so it is included in scene save/load.
    ///
    /// Call this for every component `T` that should survive a round-trip through a
    /// `.ron` scene file. The `name` must be unique across all registered types and is
    /// used as the key in [`crate::prefab::EntityDef::components`].
    ///
    /// `post_spawn` is an optional closure run after deserialization — useful for
    /// copying design-time fields to runtime counterparts (e.g. `initial_text` → `text`).
    ///
    /// # Example
    /// ```rust,no_run
    /// use engine::App;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Clone, Serialize, Deserialize)]
    /// struct Health { max: f32 }
    ///
    /// let mut app = App::new();
    /// app.register_serde_component::<Health>("Health", None);
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn register_serde_component<T>(
        &mut self,
        name: impl Into<String>,
        post_spawn: Option<Box<dyn Fn(&mut World, Entity) + Send + Sync>>,
    ) where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + Sync + 'static,
    {
        let name: String = name.into();
        // Convert Box → Arc so the closure can be cloned into the replay thunk below.
        let ps: Option<std::sync::Arc<dyn Fn(&mut World, Entity) + Send + Sync>> =
            post_spawn.map(std::sync::Arc::from);
        Self::do_register_serde_component::<T>(&mut self.world, name.clone(), ps.clone());

        // Record a replay thunk: after a scene Replace resets the World, this closure
        // re-inserts the serde registration so scene save/load keeps working.
        self.world_registrars.push(Box::new(move |world| {
            Self::do_register_serde_component::<T>(world, name.clone(), ps.clone());
        }));
    }

    /// Shared implementation for the immediate registration and scene-reset replay.
    #[allow(clippy::type_complexity)]
    fn do_register_serde_component<T>(
        world: &mut World,
        name: String,
        post_spawn: Option<std::sync::Arc<dyn Fn(&mut World, Entity) + Send + Sync>>,
    ) where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + Sync + 'static,
    {
        if let Some(registry) = world.resource_mut::<crate::prefab::SerdeComponentRegistry>() {
            registry.register_arc::<T>(name, post_spawn);
        }
    }
}
