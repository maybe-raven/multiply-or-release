
use bevy::{
    prelude::*,
    sprite::{Material2d, MaterialMesh2dBundle},
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::*;
use panel_plugin::PanelPlugin;
use utils::{Participant, UtilsPlugin};

mod panel_plugin;
mod utils;

const WINDOW_TITLE: &str = "Multiply or Release";

fn main() {
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: WINDOW_TITLE.to_string(),
            ..default()
        }),
        ..default()
    };
    App::new()
        .add_plugins(DefaultPlugins.set(window_plugin))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins((UtilsPlugin, PanelPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Name::new("Camera"), Camera2dBundle::default()));
}

#[derive(Component)]
/// Marker to mark this entity as a tile.
struct Tile;
#[derive(Bundle)]
/// Component bundle for each of the individual tiles on the battle field.
struct TileBundle<M: Material2d> {
    /// Marker to mark this entity as a tile.
    marker: Tile,
    /// Bevy rendering component used to display the tile.
    mesh: MaterialMesh2dBundle<M>,
    /// Rapier collider component. We'll mark this as sensor and won't add a rigidbody to this
    /// entity because we don't actually want the physics engine to move itl.
    collider: Collider,
    /// The game participant that owns this tile.
    owner: Participant,
}

#[derive(Component)]
struct Bullet;
#[derive(Bundle)]
/// Component bundle for the bullets that the turrets fire.
struct BulletBundle<M: Material2d> {
    /// Marker to mark this entity as a bullet.
    marker: Bullet,
    /// Bevy rendering component used to display the bullet.
    mesh: MaterialMesh2dBundle<M>,
    /// Rapier collider component.
    collider: Collider,
    /// Rapier rigidbody component, used by the physics engine to move the entity.
    rigidbody: RigidBody,
    /// The game participant that owns this bullet.
    owner: Participant,
    /// Some text component for bevy to render the text onto the ball
    /// (We're not sure exact how this would be done at the moment).
    _text: (),
}

#[derive(Component)]
/// Marker to indicate the entity is a turret head.
struct TurretHead;
#[derive(Bundle)]
/// Component bundle for the turret head (the little ball that sits on the top of the turret to
/// show its charge level and never moves).
struct TurretHeadBundle<M: Material2d> {
    /// Marker to indicate that this is a turret head.
    th: TurretHead,
    /// Bevy rendering component used to display the ball.
    mesh: MaterialMesh2dBundle<M>,
    /// A sensor collider to detect when this turret is hit by a bullet.
    collider: Collider,
    /// The game participant that owns this ball.
    owner: Participant,
    /// Some text component for bevy to render the text onto the ball
    /// (We're not sure exact how this would be done at the moment).
    _text: (),
}

/// Component for a turret.
#[derive(Component)]
#[allow(dead_code)]
struct Turret {
    /// The angle offset in degrees of the direction that the turret barrel is pointing.
    barrel_offset: f32,
    /// The direction that the barrel would be pointing in with an offset_angle of 0.
    base_direction: Vec2,
}
/// Component bundle for a turret.
#[derive(Bundle)]
#[allow(dead_code)]
struct TurretBundle<M: Material2d> {
    /// Bevy rendering component used to display the ball.
    mesh: MaterialMesh2dBundle<M>,
    /// The game participant that owns this ball.
    owner: Participant,
    /// Variables for the functionality of the turret.
    turret: Turret,
}
