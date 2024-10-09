use battlefield::BattlefieldPlugin;
use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_hanabi::prelude::*;
use bevy_rapier2d::prelude::*;
use panel_plugin::PanelPlugin;
use ui::UIPlugin;
use utils::{Participant, UtilsPlugin};

mod battlefield;
mod collision_groups;
mod debug_utils;
mod panel_plugin;
mod ui;
mod utils;

const WINDOW_TITLE: &str = "Multiply or Release";

fn main() {
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: WINDOW_TITLE.to_string(),
            mode: bevy::window::WindowMode::BorderlessFullscreen,
            ..default()
        }),
        ..default()
    };
    App::new()
        .add_plugins(DefaultPlugins.set(window_plugin))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(HanabiPlugin)
        .add_plugins((UtilsPlugin, PanelPlugin, BattlefieldPlugin, UIPlugin))
        // .add_plugins(debug_utils::DebugUtilsPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Name::new("Camera"),
        Camera2dBundle {
            camera: Camera {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
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
