use battlefield::BattlefieldPlugin;
use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_rapier2d::prelude::*;
use panel_plugin::PanelPlugin;
use participants::ParticipantsPlugin;
use ui::UIPlugin;

mod battlefield;
mod collision_groups;
mod panel_plugin;
mod participants;
mod ui;

#[cfg(feature = "dev")]
mod debug_utils;
#[cfg(not(target_family = "wasm"))]
mod effects;

const WINDOW_TITLE: &str = "Multiply or Release";

fn main() {
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: WINDOW_TITLE.to_string(),
            // Bind to canvas included in `index.html`
            canvas: Some("#bevy".to_owned()),
            fit_canvas_to_parent: true,
            // Tells wasm not to override default event handling, like F5 and Ctrl+R
            prevent_default_event_handling: false,
            mode: bevy::window::WindowMode::BorderlessFullscreen,
            ..default()
        }),
        ..default()
    };
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(window_plugin))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins((ParticipantsPlugin, PanelPlugin, BattlefieldPlugin, UIPlugin))
        .add_systems(Startup, setup);
    #[cfg(not(target_family = "wasm"))]
    app.add_plugins(effects::EffectsPlugin);
    #[cfg(feature = "dev")]
    app.add_plugins(debug_utils::DebugUtilsPlugin);
    app.run();
}

fn setup(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::BLACK));
    commands.spawn((
        Name::new("Camera"),
        Camera2dBundle {
            projection: OrthographicProjection {
                far: 1000.0,
                near: -1000.0,
                scaling_mode: ScalingMode::AutoMin {
                    min_width: 1280.0,
                    min_height: 720.0,
                },
                ..default()
            },
            ..default()
        },
    ));
}
