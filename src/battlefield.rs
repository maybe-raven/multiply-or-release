#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use crate::{
    collision_groups::{self, all_new_bullets_except},
    panel_plugin::{TriggerEvent, TriggerType},
    participants::{Participant, ParticipantMap, TILE_COLORS},
};
use bevy::{color::palettes::css, prelude::*, sprite::Mesh2dHandle, time::Stopwatch};
use bevy_rapier2d::prelude::*;
use std::{
    collections::VecDeque,
    f32::consts::{FRAC_PI_2, PI},
};

// Constants {{{

#[cfg(not(target_family = "wasm"))]
const TILE_COUNT: usize = 100;
#[cfg(target_family = "wasm")]
const TILE_COUNT: usize = 69;
const TILE_DIMENSION: f32 = BATTLEFIELD_HALF_WIDTH / TILE_COUNT as f32;
pub const BATTLEFIELD_HALF_WIDTH: f32 = 360.0;
const BATTLEFIELD_BOUNDARY_HALF_WIDTH: f32 = 50.0;

#[cfg(not(target_family = "wasm"))]
const BOOSTED_TURRET_CHARGE_VALUE: u64 = 16;
#[cfg(target_family = "wasm")]
const BOOSTED_TURRET_CHARGE_VALUE: u64 = 8;
/// The time in seconds after getting hit that a turret's charge will reset to 1 whenever it fires
/// instead of [ `BOOSTED_TURRET_CHARGE_VALUE` ]
const TURRET_BOOST_COOLDOWN: f32 = 5.0;
const TURRET_POSITION: f32 = 330.0;
const TURRET_HEAD_COLOR: Color = Color::Srgba(css::DARK_GRAY);
const TURRET_HEAD_THICNESS: f32 = 3.0;
const TURRET_HEAD_LENGTH: f32 = 50.0;
const TURRET_ROTATION_SPEED: f32 = 0.75;

const MULTI_SHOT_CHARGE_OFFSET: u64 = 8;

/// The width of a rectangular area at the corner where the `NEW_BULLET` tag will not be dropped.
const NEW_BULLET_PHASE_RANGE: f32 = 2.0 * (BATTLEFIELD_HALF_WIDTH - TURRET_POSITION);
const BULLET_TEXT_COLOR: Color = Color::BLACK;
const BULLET_TEXT_FONT_SIZE_ASPECT: f32 = 0.5;
const BULLET_MINIMUM_TEXT_SIZE: f32 = 8.0;
const BULLET_SIZE_FACTOR: f32 = 2.0;
const BULLET_MASS_FACTOR: f32 = 2.0;
const BULLET_RESTITUTION_COEFFICIENT: f32 = 0.75;
const CHARGED_SHOT_BULLET_SPEED: f32 = 250.0;
const BURST_SHOT_BULLET_SPEED: f32 = 500.0;
/// Time in seconds the turret will stop firing for after firing a charged shot.
const CHARGED_SHOT_COOLDOWN: f32 = 0.5;

// Z-index
const TILE_Z: f32 = -1.0;
const BULLET_BALL_Z: f32 = -1.0;
const BULLET_TEXT_Z: f32 = 3.0;
// Turret head is a child of turret, which inherits the z position as well, so the local z of the
// head needs to be negative to put it behind the main turret.
const TURRET_HEAD_Z: f32 = -1.0;
const TURRET_PLATFORM_Z: f32 = -1.0;

// }}}

pub struct BattlefieldPlugin;
impl Plugin for BattlefieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<EliminationEvent>()
            .add_event::<RestartEvent>()
            .add_event::<TileHitEvent>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    rotate_turret,
                    handle_bullet_tile_collision,
                    handle_bullet_turret_collision.after(handle_bullet_tile_collision),
                    handle_trigger_events
                        .after(handle_bullet_turret_collision)
                        .run_if(on_event::<TriggerEvent>().or_else(on_event::<RestartEvent>())),
                    update_charge_level.after(handle_trigger_events),
                    update_charge_ball.after(update_charge_level),
                    handle_elimination
                        .run_if(on_event::<EliminationEvent>())
                        .after(update_charge_level),
                ),
            )
            .add_systems(PostUpdate, restart.run_if(on_event::<RestartEvent>()))
            .add_systems(
                FixedUpdate,
                (
                    update_bullets_solver_groups.before(fire_shots),
                    fire_shots
                        .run_if(game_is_going)
                        .after(handle_trigger_events),
                ),
            );
    }
}

#[derive(Clone, Copy, Event, Default)]
pub struct RestartEvent;
#[derive(Clone, Copy, Event)]
pub struct EliminationEvent {
    pub participant: Participant,
}
#[cfg_attr(target_family = "wasm", allow(dead_code))]
#[derive(Clone, Copy, Event)]
pub struct TileHitEvent {
    pub position: Vec3,
    pub participant: Participant,
    pub bullet_velocity: Vec2,
}
impl EliminationEvent {
    fn new(participant: Participant) -> Self {
        Self { participant }
    }
}
#[derive(Resource)]
pub struct SurvivorCount(pub u8);
impl Default for SurvivorCount {
    fn default() -> Self {
        Self(4)
    }
}
#[derive(Component, Clone, Copy)]
struct BattlefieldRoot;
#[derive(Component, Clone, Copy)]
struct TileRoot;
/// Marker to mark this entity as a tile.
#[derive(Component, Clone, Copy)]
pub struct Tile;
/// Component bundle for each of the individual tiles on the battle field.
#[derive(Bundle)]
struct TileBundle {
    /// Markers to mark this entity as a tile, a sensor collider, and a trigger for collision
    /// events.
    markers: (Tile, Sensor),
    /// Bevy rendering component used to display the tile.
    sprite_bundle: SpriteBundle,
    /// Rapier collider component. We'll mark this as sensor and won't add a rigidbody to this
    /// entity because we don't actually want the physics engine to move itl.
    collider: Collider,
    collision_groups: CollisionGroups,
    /// The game participant that owns this tile.
    owner: Participant,
    name: Name,
}
impl TileBundle {
    fn new(owner: Participant, color: impl Into<Color>, x: f32, y: f32) -> Self {
        Self {
            markers: (Tile, Sensor),
            sprite_bundle: SpriteBundle {
                transform: Transform {
                    translation: Vec3::new(x, y, TILE_Z),
                    scale: Vec3::new(TILE_DIMENSION, TILE_DIMENSION, 1.0),
                    rotation: Quat::IDENTITY,
                },
                sprite: Sprite {
                    color: color.into(),
                    ..default()
                },
                ..default()
            },
            collider: Collider::cuboid(0.5, 0.5),
            collision_groups: CollisionGroups::new(
                collision_groups::tile(owner),
                collision_groups::all_bullets_except(owner)
                    | collision_groups::all_new_bullets_except(owner),
            ),
            owner,
            name: Name::new("Tile"),
        }
    }
}
#[derive(Resource, Default, Clone)]
struct TurretStopwatch(Stopwatch);
impl TurretStopwatch {
    fn get(&self) -> f32 {
        (self.0.elapsed_secs() * TURRET_ROTATION_SPEED) % (2.0 * PI)
    }
}
#[derive(Component, Deref, Clone, Copy)]
struct ChargeBallLink(Entity);
#[derive(Debug, Component, Clone, Copy)]
struct Charge {
    value: u64,
    level: u64,
}
impl Default for Charge {
    fn default() -> Self {
        Self {
            value: BOOSTED_TURRET_CHARGE_VALUE,
            level: Self::calculate_level(BOOSTED_TURRET_CHARGE_VALUE),
        }
    }
}
impl Charge {
    fn calculate_level(value: u64) -> u64 {
        (value as f64).log2().ceil() as u64 + 1
    }
    fn from_value(value: u64) -> Self {
        let mut v = Self { value, level: 1 };
        v.update_level();
        v
    }
    fn update_level(&mut self) {
        self.level = Self::calculate_level(self.value);
    }
    fn multiply(&mut self, factor: u8) {
        if let Some(value) = self.value.checked_mul(factor as u64) {
            self.value = value;
        } else {
            self.value = u64::MAX;
        }
    }
    fn reset_boosted(&mut self) {
        self.value = BOOSTED_TURRET_CHARGE_VALUE;
        self.update_level();
    }
    fn reset(&mut self) {
        self.value = 1;
        self.level = 1;
    }
    fn get_scale(&self) -> f32 {
        self.level as f32 * BULLET_SIZE_FACTOR
    }
    fn get_mass(&self) -> f32 {
        self.value as f32 * BULLET_MASS_FACTOR
    }
}
#[derive(Bundle)]
struct ChargeBallBundle {
    matmesh: ColorMesh2dBundle,
    name: Name,
}
impl ChargeBallBundle {
    fn new(mesh: Mesh2dHandle, material: Handle<ColorMaterial>) -> Self {
        Self {
            matmesh: ColorMesh2dBundle {
                transform: Transform::from_xyz(0.0, 0.0, BULLET_BALL_Z),
                mesh,
                material,
                ..default()
            },
            name: Name::new("Charge Ball"),
        }
    }
}
#[derive(Resource, Deref)]
struct BulletMesh(Mesh2dHandle);
#[derive(Clone, Copy, Component)]
struct Bullet;
#[derive(Clone, Copy, Component)]
struct NewBullet;
/// Component bundle for the bullets that the turrets fire.
#[derive(Bundle)]
struct BulletBundle {
    /// Marker to mark this entity as a bullet.
    markers: (
        Bullet,
        NewBullet,
        GravityScale,
        Friction,
        Restitution,
        LockedAxes,
        ActiveEvents,
    ),
    charge: Charge,
    link: ChargeBallLink,
    /// Rapier collider component.
    collider: Collider,
    collision_groups: CollisionGroups,
    solver_groups: SolverGroups,
    collider_scale: ColliderScale,
    velocity: Velocity,
    /// Rapier rigidbody component, used by the physics engine to move the entity.
    rigidbody: RigidBody,
    mass: ColliderMassProperties,
    /// The game participant that owns this bullet.
    owner: Participant,
    text_bundle: Text2dBundle,
    name: Name,
}
impl BulletBundle {
    fn new(
        owner: Participant,
        position: Vec2,
        ball: Entity,
        charge: Charge,
        firing_angle: f32,
        bullet_speed: f32,
    ) -> Self {
        let direction = Vec2::from_angle(firing_angle);
        Self {
            owner,
            name: Name::new("Bullet"),
            charge,
            link: ChargeBallLink(ball),
            markers: (
                Bullet,
                NewBullet,
                GravityScale(0.0),
                Friction {
                    coefficient: 0.0,
                    combine_rule: CoefficientCombineRule::Min,
                },
                Restitution {
                    coefficient: BULLET_RESTITUTION_COEFFICIENT,
                    combine_rule: CoefficientCombineRule::Max,
                },
                LockedAxes::ROTATION_LOCKED,
                ActiveEvents::COLLISION_EVENTS,
            ),
            collider: Collider::ball(1.0),
            collision_groups: CollisionGroups::new(
                collision_groups::new_bullet(owner),
                collision_groups::BATTLEFIELD_ROOT
                    | collision_groups::ALL_BULLETS
                    | collision_groups::ALL_NEW_BULLETS
                    | collision_groups::ALL_TURRETS
                    | collision_groups::all_tiles_except(owner),
            ),
            solver_groups: SolverGroups::new(
                collision_groups::new_bullet(owner),
                collision_groups::BATTLEFIELD_ROOT
                    | collision_groups::ALL_BULLETS
                    | collision_groups::all_new_bullets_except(owner),
            ),
            collider_scale: ColliderScale::Absolute(Vect::splat(1.0)),
            velocity: Velocity::linear(direction * bullet_speed),
            rigidbody: RigidBody::Dynamic,
            mass: ColliderMassProperties::Mass(charge.get_mass()),
            text_bundle: Text2dBundle {
                transform: Transform::from_translation(position.extend(BULLET_TEXT_Z)),
                text: Text::from_section(
                    "",
                    TextStyle {
                        font: Default::default(),
                        font_size: BULLET_SIZE_FACTOR,
                        color: BULLET_TEXT_COLOR,
                    },
                ),
                ..default()
            },
        }
    }
}
#[derive(Debug, Clone, Copy)]
enum ShotType {
    Charged,
    Multi,
}
#[derive(Component)]
struct Turret {
    firing_queue: VecDeque<(ShotType, Charge)>,
    last_hit_timestamp: f32,
    last_charged_shot_timestamp: f32,
}
impl Default for Turret {
    fn default() -> Self {
        Self {
            firing_queue: VecDeque::new(),
            last_hit_timestamp: -TURRET_BOOST_COOLDOWN,
            last_charged_shot_timestamp: -CHARGED_SHOT_COOLDOWN,
        }
    }
}
#[derive(Bundle)]
struct TurretBundle {
    firing_queue: Turret,
    charge: Charge,
    link: ChargeBallLink,
    platform: TurretPlatformLink,
    text_bundle: Text2dBundle,
    owner: Participant,
    rb: RigidBody,
    collider: Collider,
    collision_groups: CollisionGroups,
    collider_scale: ColliderScale,
    active_events: ActiveEvents,
    name: Name,
}
impl TurretBundle {
    fn new(owner: Participant, x: f32, y: f32, ball: Entity, platform: Entity) -> Self {
        Self {
            owner,
            name: Name::new(format!("Turret: {}", owner)),
            firing_queue: Turret::default(),
            charge: Charge::default(),
            link: ChargeBallLink(ball),
            platform: TurretPlatformLink(platform),
            rb: RigidBody::Fixed,
            collider: Collider::ball(1.0),
            collision_groups: CollisionGroups::new(
                collision_groups::turret(owner),
                collision_groups::ALL_BULLETS | collision_groups::all_new_bullets_except(owner),
            ),
            collider_scale: ColliderScale::Absolute(Vect::splat(1.0)),
            active_events: ActiveEvents::COLLISION_EVENTS,
            text_bundle: Text2dBundle {
                transform: Transform::from_xyz(x, y, BULLET_TEXT_Z),
                text: Text::from_section(
                    "",
                    TextStyle {
                        font: Default::default(),
                        font_size: BULLET_SIZE_FACTOR,
                        color: BULLET_TEXT_COLOR,
                    },
                ),
                ..default()
            },
        }
    }
}
/// Marker to indicate the entity is a turret head.
#[derive(Component)]
struct TurretBarrel;
#[derive(Bundle)]
struct TurretBarrelBundle {
    /// Marker to indicate that this is a turret head.
    marker: TurretBarrel,
    sprite_bundle: SpriteBundle,
    name: Name,
}
impl TurretBarrelBundle {
    fn new() -> Self {
        Self {
            marker: TurretBarrel,
            name: Name::new("Turret Barrel"),
            sprite_bundle: SpriteBundle {
                sprite: Sprite {
                    color: TURRET_HEAD_COLOR,
                    ..default()
                },
                transform: Transform {
                    translation: Vec3::new(TURRET_HEAD_LENGTH / 2.0, 0.0, TURRET_HEAD_Z),
                    scale: Vec3::new(TURRET_HEAD_LENGTH, TURRET_HEAD_THICNESS, 1.0),
                    rotation: Quat::IDENTITY,
                },
                ..default()
            },
        }
    }
}
#[derive(Component)]
struct TurretPlatformLink(Entity);
/// Component for a turret.
#[derive(Component, Default)]
struct BarrelOffset(f32);
/// Component bundle for a turret.
#[derive(Bundle, Default)]
struct TurretPlatformBundle {
    barrel_offset: BarrelOffset,
    spatial: SpatialBundle,
    name: Name,
}
impl TurretPlatformBundle {
    fn new(base_offset: f32) -> Self {
        Self {
            name: Name::new("Turret Platform"),
            barrel_offset: BarrelOffset(base_offset),
            spatial: SpatialBundle::from_transform(Transform::from_xyz(
                0.0,
                0.0,
                TURRET_PLATFORM_Z,
            )),
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<ParticipantMap<Handle<ColorMaterial>>>,
) {
    commands.insert_resource(TurretStopwatch::default());
    commands.insert_resource(SurvivorCount::default());
    const OFFSET: f32 = BATTLEFIELD_HALF_WIDTH + BATTLEFIELD_BOUNDARY_HALF_WIDTH;
    let horizontal_cuboid = Collider::cuboid(
        BATTLEFIELD_HALF_WIDTH + BATTLEFIELD_BOUNDARY_HALF_WIDTH * 2.0,
        BATTLEFIELD_BOUNDARY_HALF_WIDTH,
    );
    let vertical_cuboid = Collider::cuboid(
        BATTLEFIELD_BOUNDARY_HALF_WIDTH,
        BATTLEFIELD_HALF_WIDTH + BATTLEFIELD_BOUNDARY_HALF_WIDTH * 2.0,
    );
    let collider = Collider::compound(vec![
        (Vect::new(OFFSET, 0.0), 0.0, vertical_cuboid.clone()),
        (Vect::new(-OFFSET, 0.0), 0.0, vertical_cuboid.clone()),
        (Vect::new(0.0, OFFSET), 0.0, horizontal_cuboid.clone()),
        (Vect::new(0.0, -OFFSET), 0.0, horizontal_cuboid.clone()),
    ]);
    let root = commands
        .spawn((
            Name::new("Battlefield Root"),
            BattlefieldRoot,
            RigidBody::Fixed,
            CollisionGroups::new(
                collision_groups::BATTLEFIELD_ROOT,
                collision_groups::ALL_BULLETS | collision_groups::ALL_NEW_BULLETS,
            ),
            Restitution {
                coefficient: 1.0,
                combine_rule: CoefficientCombineRule::Max,
            },
            collider,
            SpatialBundle::default(),
        ))
        .id();
    let tile_root = commands
        .spawn((Name::new("Tile Root"), (TileRoot, SpatialBundle::default())))
        .set_parent(root)
        .id();
    setup_tiles(&mut commands, tile_root);
    let mesh = Mesh2dHandle(meshes.add(Circle::new(1.0)));
    let maps = setup_turrets(&mut commands, root, mesh.clone(), &materials);
    commands.insert_resource(maps);
    commands.insert_resource(BulletMesh(mesh));
}
fn rotate_turret(
    time: Res<Time>,
    mut stopwatch: ResMut<TurretStopwatch>,
    mut turrets: Query<(&mut Transform, &BarrelOffset)>,
) {
    stopwatch.0.tick(time.delta());
    let angle_offset = stopwatch.get();
    for (mut transform, &BarrelOffset(base_offset)) in &mut turrets {
        *transform = transform.with_rotation(Quat::from_rotation_z(base_offset + angle_offset));
    }
}
fn update_charge_level(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Charge, &Participant, Option<&Turret>), Changed<Charge>>,
    mut event_writer: EventWriter<EliminationEvent>,
) {
    for (entity, mut charge, &participant, firing_queue) in &mut query {
        if charge.value > 0 {
            charge.update_level();
        } else if firing_queue.is_some() {
            event_writer.send(EliminationEvent::new(participant));
        } else {
            commands.entity(entity).despawn_recursive();
        }
    }
}
fn setup_tiles(commands: &mut Commands, tile_root: Entity) {
    for i in 0..TILE_COUNT {
        let x = TILE_DIMENSION / 2.0 + i as f32 * TILE_DIMENSION;
        for j in 0..TILE_COUNT {
            let y = TILE_DIMENSION / 2.0 + j as f32 * TILE_DIMENSION;
            commands
                .spawn(TileBundle::new(Participant::A, TILE_COLORS.a, x, y))
                .set_parent(tile_root);
            commands
                .spawn(TileBundle::new(Participant::B, TILE_COLORS.b, -x, y))
                .set_parent(tile_root);
            commands
                .spawn(TileBundle::new(Participant::C, TILE_COLORS.c, x, -y))
                .set_parent(tile_root);
            commands
                .spawn(TileBundle::new(Participant::D, TILE_COLORS.d, -x, -y))
                .set_parent(tile_root);
        }
    }
}
fn setup_turrets(
    commands: &mut Commands,
    root: Entity,
    mesh: Mesh2dHandle,
    materials: &ParticipantMap<Handle<ColorMaterial>>,
) -> ParticipantMap<Entity> {
    let mut spawn_turret = |owner: Participant, base_offset: f32, x: f32, y: f32| {
        let ball = commands
            .spawn(ChargeBallBundle::new(
                mesh.clone(),
                materials[owner].clone(),
            ))
            .id();
        let platform = commands
            .spawn(TurretPlatformBundle::new(base_offset))
            .set_parent(root)
            .id();
        commands
            .spawn(TurretBarrelBundle::new())
            .set_parent(platform);
        commands
            .spawn(TurretBundle::new(owner, x, y, ball, platform))
            .set_parent(root)
            .push_children(&[ball, platform])
            .id()
    };
    let a = spawn_turret(Participant::A, PI, TURRET_POSITION, TURRET_POSITION);
    let b = spawn_turret(
        Participant::B,
        -FRAC_PI_2,
        -TURRET_POSITION,
        TURRET_POSITION,
    );
    let c = spawn_turret(Participant::C, FRAC_PI_2, TURRET_POSITION, -TURRET_POSITION);
    let d = spawn_turret(Participant::D, 0.0, -TURRET_POSITION, -TURRET_POSITION);
    ParticipantMap::new(a, b, c, d)
}
fn update_charge_ball(
    mut balls: Query<
        (
            &mut ColliderScale,
            Option<&mut ColliderMassProperties>,
            &mut Text,
            &Charge,
            &ChargeBallLink,
            Entity,
        ),
        Or<(Changed<Charge>, Added<Charge>)>,
    >,
    turret_query: Query<(), With<Turret>>,
    mut transform_query: Query<&mut Transform>,
) {
    for (mut collider_scale, mass_properties, mut text, charge, &ChargeBallLink(link), entity) in
        &mut balls
    {
        let mut scale = charge.get_scale();
        if scale < BULLET_MINIMUM_TEXT_SIZE && turret_query.get(entity).is_ok() {
            scale = BULLET_MINIMUM_TEXT_SIZE;
        }
        let new_scale = ColliderScale::Absolute(Vect::splat(scale));
        if *collider_scale != new_scale {
            *collider_scale = new_scale;
        }
        if let Some(mut mass_properties) = mass_properties {
            *mass_properties = ColliderMassProperties::Mass(charge.get_mass());
        }
        let mut ball_transform = transform_query.get_mut(link).unwrap();
        ball_transform.scale.x = scale;
        ball_transform.scale.y = scale;
        let diameter = scale * 2.0;
        let section = &mut text.sections[0];
        if diameter < BULLET_MINIMUM_TEXT_SIZE {
            section.value.clear();
        } else {
            section.value = charge.value.to_string();
            let digit_count = section.value.len() as f32;
            let full_size_horizontal = diameter * BULLET_TEXT_FONT_SIZE_ASPECT * digit_count;
            if diameter < full_size_horizontal {
                section.style.font_size = diameter / digit_count / BULLET_TEXT_FONT_SIZE_ASPECT;
            } else {
                section.style.font_size = diameter;
            }
        }
    }
}
fn update_bullets_solver_groups(
    mut commands: Commands,
    rapier: Res<RapierContext>,
    mut bullet_query: Query<
        (
            Entity,
            &mut CollisionGroups,
            &mut SolverGroups,
            &Participant,
            &Transform,
        ),
        With<NewBullet>,
    >,
) {
    for (entity, mut collision_groups, mut solver_groups, &participant, transform) in
        &mut bullet_query
    {
        if BATTLEFIELD_HALF_WIDTH - transform.translation.x.abs() < NEW_BULLET_PHASE_RANGE
            && BATTLEFIELD_HALF_WIDTH - transform.translation.y.abs() < NEW_BULLET_PHASE_RANGE
        {
            continue;
        }
        if !rapier
            .contact_pairs_with(entity)
            .any(|x| x.has_any_active_contact())
        {
            collision_groups.memberships = collision_groups::bullet(participant);
            collision_groups.filters = collision_groups::BATTLEFIELD_ROOT
                | collision_groups::ALL_BULLETS
                | collision_groups::ALL_NEW_BULLETS
                | collision_groups::ALL_TURRETS
                | collision_groups::all_tiles_except(participant);
            solver_groups.memberships = collision_groups::bullet(participant);
            solver_groups.filters = collision_groups::BATTLEFIELD_ROOT
                | collision_groups::ALL_BULLETS
                | collision_groups::ALL_NEW_BULLETS
                | collision_groups::ALL_TURRETS;
            commands.entity(entity).remove::<NewBullet>();
        }
    }
}
fn fire_shots(
    mut commands: Commands,
    mesh: Res<BulletMesh>,
    materials: Res<ParticipantMap<Handle<ColorMaterial>>>,
    turret_stopwatch: Res<TurretStopwatch>,
    mut turrets: Query<(&mut Turret, &Transform, &Participant, &TurretPlatformLink)>,
    platform_query: Query<&BarrelOffset>,
    battlefield_root: Query<Entity, With<BattlefieldRoot>>,
    time: Res<Time>,
) {
    for (mut turret, transform, &owner, &TurretPlatformLink(link)) in &mut turrets {
        if time.elapsed_seconds() - turret.last_charged_shot_timestamp < CHARGED_SHOT_COOLDOWN {
            continue;
        }
        let Some((shot_type, charge)) = turret.firing_queue.pop_back() else {
            continue;
        };
        let get_offset = |radius: f32| {
            let translation = transform.translation;
            let absx = translation.x.abs();
            let abs_offset = absx - absx.min(BATTLEFIELD_HALF_WIDTH - radius);
            Vec2::new(translation.x.signum(), translation.y.signum()) * abs_offset
        };
        let (charge, offset, bullet_speed) = match shot_type {
            ShotType::Charged => {
                let radius = charge.get_scale();
                let offset = get_offset(radius);
                turret.last_charged_shot_timestamp = time.elapsed_seconds();
                (charge, offset, CHARGED_SHOT_BULLET_SPEED)
            }
            ShotType::Multi => {
                let shot_value = match charge.level.checked_sub(MULTI_SHOT_CHARGE_OFFSET) {
                    None | Some(0) => 1,
                    Some(value) => value,
                };
                let shot = Charge::from_value(shot_value);
                let radius = shot.get_scale();
                let offset = get_offset(radius);
                let mut charge = charge;
                match charge.value.checked_sub(shot.value) {
                    None | Some(0) => (),
                    Some(remaining_value) => {
                        charge.value = remaining_value;
                        charge.update_level();
                        turret.firing_queue.push_back((shot_type, charge));
                    }
                }
                (shot, offset, BURST_SHOT_BULLET_SPEED)
            }
        };
        let &BarrelOffset(base_angle) = platform_query.get(link).unwrap();
        let ball = commands
            .spawn(ChargeBallBundle::new(
                mesh.clone(),
                materials[owner].clone(),
            ))
            .id();
        commands
            .spawn(BulletBundle::new(
                owner,
                transform.translation.xy() - offset,
                ball,
                charge,
                turret_stopwatch.get() + base_angle,
                bullet_speed,
            ))
            .set_parent(battlefield_root.single())
            .add_child(ball);
    }
}
fn handle_trigger_events(
    mut trigger_events: EventReader<TriggerEvent>,
    mut restart_events: EventReader<RestartEvent>,
    turret_entities: Res<ParticipantMap<Entity>>,
    mut turret_query: Query<(&mut Charge, &mut Turret)>,
    time: Res<Time>,
) {
    if !restart_events.is_empty() {
        restart_events.clear();
        trigger_events.clear();
    }
    for event in trigger_events.read() {
        let entity = turret_entities[event.participant];
        let Ok((mut charge, mut turret)) = turret_query.get_mut(entity) else {
            continue;
        };
        match event.trigger_type {
            TriggerType::Multiply(factor) => charge.multiply(factor),
            TriggerType::BurstShot => {
                turret.firing_queue.push_front((ShotType::Multi, *charge));
                if time.elapsed_seconds() - turret.last_hit_timestamp > TURRET_BOOST_COOLDOWN {
                    charge.reset_boosted();
                } else {
                    charge.reset();
                }
            }
            TriggerType::ChargedShot => {
                turret.firing_queue.push_front((ShotType::Charged, *charge));
                if time.elapsed_seconds() - turret.last_hit_timestamp > TURRET_BOOST_COOLDOWN {
                    charge.reset_boosted();
                } else {
                    charge.reset();
                }
            }
        }
    }
}
fn handle_bullet_turret_collision(
    mut collision_event_reader: EventReader<CollisionEvent>,
    mut bullet_query: Query<(&Participant, &mut Charge), With<Bullet>>,
    mut turret_query: Query<
        (&Participant, &mut Charge, &mut Turret),
        (With<Turret>, Without<Bullet>),
    >,
    time: Res<Time>,
) {
    for event in collision_event_reader.read() {
        let &CollisionEvent::Started(a, b, _) = event else {
            continue;
        };
        let (&bullet_owner, mut bullet_charge) = if let Ok(x) = bullet_query.get_mut(a) {
            x
        } else if let Ok(x) = bullet_query.get_mut(b) {
            x
        } else {
            continue;
        };
        let (&turret_owner, mut turret_charge, mut turret) = if let Ok(x) = turret_query.get_mut(a)
        {
            x
        } else if let Ok(x) = turret_query.get_mut(b) {
            x
        } else {
            continue;
        };
        if turret_owner == bullet_owner {
            continue;
        }
        let min_value = bullet_charge.value.min(turret_charge.value);
        bullet_charge.value -= min_value;
        turret_charge.value -= min_value;
        turret.last_hit_timestamp = time.elapsed_seconds();
    }
}
fn handle_elimination(
    mut commands: Commands,
    mut events: EventReader<EliminationEvent>,
    mut survivor_count: ResMut<SurvivorCount>,
    mut survivors: ResMut<ParticipantMap<bool>>,
    participant_entity_query: Query<(Entity, &Participant), (Without<Tile>, Without<Bullet>)>,
) {
    for event in events.read() {
        survivors[event.participant] = false;
        survivor_count.0 -= 1;
        for (entity, &participant) in &participant_entity_query {
            if participant == event.participant {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}
fn handle_bullet_tile_collision(
    mut collision_events: EventReader<CollisionEvent>,
    mut tile_hit_events: EventWriter<TileHitEvent>,
    mut bullet_query: Query<(&Participant, &mut Charge, &Velocity), With<Bullet>>,
    mut tile_query: Query<
        (
            &mut Participant,
            &mut Sprite,
            &mut CollisionGroups,
            &GlobalTransform,
        ),
        (With<Tile>, Without<Bullet>),
    >,
) {
    for event in collision_events.read() {
        match event {
            &CollisionEvent::Started(a, b, _) => {
                let (&bullet_owner, mut charge, velocity) = if let Ok(x) = bullet_query.get_mut(a) {
                    x
                } else if let Ok(x) = bullet_query.get_mut(b) {
                    x
                } else {
                    continue;
                };
                let (mut tile_owner, mut sprite, mut collision_group, transform) =
                    if let Ok(x) = tile_query.get_mut(a) {
                        x
                    } else if let Ok(x) = tile_query.get_mut(b) {
                        x
                    } else {
                        continue;
                    };
                if bullet_owner == *tile_owner {
                    continue;
                }
                if charge.value == 0 {
                    continue;
                }
                *tile_owner = bullet_owner;
                sprite.color = Color::from(TILE_COLORS[bullet_owner]);
                *collision_group = CollisionGroups::new(
                    collision_groups::tile(bullet_owner),
                    collision_groups::all_bullets_except(bullet_owner)
                        | all_new_bullets_except(bullet_owner),
                );
                charge.value -= 1;
                tile_hit_events.send(TileHitEvent {
                    position: transform.translation(),
                    participant: bullet_owner,
                    bullet_velocity: velocity.linvel,
                });
            }
            CollisionEvent::Stopped(_, _, _) => (),
        }
    }
}
pub fn game_is_going(survivor_count: Res<SurvivorCount>) -> bool {
    survivor_count.0 > 1
}
fn restart(
    mut commands: Commands,
    mut survivor_count: ResMut<SurvivorCount>,
    mut survivors: ResMut<ParticipantMap<bool>>,
    mut turrets: ResMut<ParticipantMap<Entity>>,
    mut stopwatch: ResMut<TurretStopwatch>,
    materials: Res<ParticipantMap<Handle<ColorMaterial>>>,
    ball_mesh: Res<BulletMesh>,
    tile_root: Query<(Entity, &Children), With<TileRoot>>,
    garbage: Query<Entity, Or<(With<Bullet>, With<NewBullet>, With<Turret>)>>,
    root: Query<Entity, With<BattlefieldRoot>>,
) {
    survivor_count.0 = 4;
    survivors.a = true;
    survivors.b = true;
    survivors.c = true;
    survivors.d = true;
    for entity in garbage.iter() {
        commands.entity(entity).despawn_recursive();
    }
    let (tile_root_entity, tile_root_children) = tile_root.single();
    for &tile in tile_root_children.iter() {
        commands.entity(tile).despawn_recursive();
    }
    setup_tiles(&mut commands, tile_root_entity);
    *turrets = setup_turrets(
        &mut commands,
        root.single(),
        ball_mesh.0.clone(),
        &materials,
    );
    stopwatch.0.reset();
}
