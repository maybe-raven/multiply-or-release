#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use crate::{
    battlefield::{game_is_going, RestartEvent},
    collision_groups::{self, PANEL_OBSTACLES, PANEL_TRIGGER_ZONES},
    participants::{Participant, ParticipantMap},
};
use bevy::{
    color::palettes::css,
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use bevy_rapier2d::prelude::*;
use rand::{
    distributions::{DistIter, Distribution, Uniform},
    rngs::ThreadRng,
    thread_rng, Rng,
};
use std::time::Duration;

// Constants {{{

// Configurable

pub const LEFT_ROOT_X: f32 = -500.0;
pub const RIGHT_ROOT_X: f32 = 500.0;

const WALL_THICKNESS: f32 = 10.0;
const WALL_COLOR: Color = Color::srgb(0.8, 0.8, 0.8);
const ARENA_COLOR: Color = Color::Srgba(css::DARK_SLATE_GRAY);
const ARENA_HEIGHT: f32 = 700.0;
const ARENA_WIDTH: f32 = 260.0;

const TRIGGER_ZONE_Y: f32 = -250.0;
const TRIGGER_ZONE_HEIGHT: f32 = 40.0;
/// The color of the center trigger zone.
const TRIGGER_ZONE_COLOR_0: Color = Color::Srgba(css::ALICE_BLUE);
/// The color of the trigger zones to the left and right of center.
const TRIGGER_ZONE_COLOR_1: Color = Color::Srgba(css::LIGHT_PINK);
/// The color of the outer trigger zones.
const TRIGGER_ZONE_COLOR_2: Color = Color::Srgba(css::LIGHT_SKY_BLUE);
const TRIGGER_ZONE_TEXT_COLOR: Color = Color::BLACK;
const TRIGGER_ZONE_TEXT_SIZE: f32 = 12.0;

const CIRCLE_RADIUS: f32 = 10.0;
const CIRCLE_COLOR: Color = Color::srgb(0.8, 0.8, 0.8);
const CIRCLE_PYRAMID_VERTICAL_OFFSET: f32 = 250.0;
const CIRCLE_PYRAMID_VERTICAL_COUNT: usize = 5;
const CIRCLE_PYRAMID_VERTICAL_GAP: f32 = 8.0;
const CIRCLE_PYRAMID_HORIZONTAL_GAP: f32 = 45.0;

const TRIGGER_ZONE_DIVIDER_COLOR: Color = Color::srgb(0.8, 0.8, 0.8);
const TRIGGER_ZONE_DIVIDER_HEIGHT_OFFSET: f32 = 2.5;
const TRIGGER_ZONE_DIVIDER_RADIUS: f32 = 2.5;

const CIRCLE_GRID_VERTICAL_OFFSET: f32 = 70.0;
const CIRCLE_GRID_VERTICAL_COUNT: usize = 8;
const CIRCLE_GRID_VERTICAL_GAP: f32 = 15.0;
const CIRCLE_GRID_HORIZONTAL_GAP: f32 = 28.0;
const CIRCLE_GRID_HORIZONTAL_HALF_COUNT_EVEN_ROW: usize = 2;
const CIRCLE_GRID_HORIZONTAL_HALF_COUNT_ODD_ROW: usize = 3;

pub const WORKER_BALL_RADIUS: f32 = 5.0;
pub const WORKER_BALL_SPAWN_Y: f32 = 320.0;
const WORKER_BALL_RESTITUTION_COEFFICIENT: f32 = 0.5;
const WORKER_BALL_SPAWN_TIMER_SECS: f32 = 10.0;
#[cfg(not(target_family = "wasm"))]
const WORKER_BALL_SPAWN_TIMER_INIT_SECS: f32 =
    WORKER_BALL_SPAWN_TIMER_SECS - crate::effects::TRAIL_LIFETIME;
#[cfg(target_family = "wasm")]
const WORKER_BALL_SPAWN_TIMER_INIT_SECS: f32 = WORKER_BALL_SPAWN_TIMER_SECS - 0.1;
pub const WORKER_BALL_COUNT_MAX: usize = 6;
const WORKER_BALL_GRAVITY_SCALE: f32 = 15.0;

// Z-index
const WALL_Z: f32 = -4.0;
const ARENA_Z: f32 = -3.0;
const CIRCLE_Z: f32 = -1.0;
const TRIGGER_ZONE_Z: f32 = -2.0;
const TRIGGER_ZONE_DIVIDER_Z: f32 = -1.0;
const TRIGGER_ZONE_TEXT_OFFSET_Z: f32 = -1.0;
const WORKER_BALL_Z: f32 = 1.0;

// Calculated
const WALL_HEIGHT: f32 = ARENA_HEIGHT + 2.0 * WALL_THICKNESS;
const WALL_WIDTH: f32 = ARENA_WIDTH + 2.0 * WALL_THICKNESS;
const ARENA_HEIGHT_FRAC_2: f32 = ARENA_HEIGHT / 2.0;
const ARENA_WIDTH_FRAC_2: f32 = ARENA_WIDTH / 2.0;
const ARENA_WIDTH_FRAC_5: f32 = ARENA_WIDTH / 5.0;
const ARENA_WIDTH_FRAC_10: f32 = ARENA_WIDTH / 10.0;

const CIRCLE_HALF_GAP: f32 = CIRCLE_PYRAMID_HORIZONTAL_GAP / 2.0;
const CIRCLE_DIAMETER: f32 = CIRCLE_RADIUS * 2.0;

const WORKER_BALL_DIAMETER: f32 = WORKER_BALL_RADIUS * 2.0;

// }}}

pub struct PanelPlugin;
impl Plugin for PanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TriggerEvent>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    spawn_workers.run_if(game_is_going.and_then(spawn_workers_condition)),
                    reset_workers.run_if(game_is_going),
                    trigger_event
                        .run_if(on_event::<CollisionEvent>().or_else(on_event::<RestartEvent>())),
                ),
            )
            .add_systems(PostUpdate, restart.run_if(on_event::<RestartEvent>()));
    }
}

#[derive(Debug, Event)]
pub struct TriggerEvent {
    pub participant: Participant,
    pub trigger_type: TriggerType,
}
#[derive(Debug, Component, Clone, Copy)]
pub enum TriggerType {
    Multiply(u8),
    BurstShot,
    ChargedShot,
}
impl std::fmt::Display for TriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Multiply(factor) => write!(f, "x{}", factor),
            Self::BurstShot => write!(f, "Release\nBurst\nShots"),
            Self::ChargedShot => write!(f, "Release\nChanged\nShots"),
        }
    }
}
#[derive(Bundle, Clone)]
struct TriggerZoneBundle {
    // {{{
    sprite_bundle: SpriteBundle,
    collider: Collider,
    collision_groups: CollisionGroups,
    trigger_type: TriggerType,
    markers: (ActiveEvents, Sensor),
    name: Name,
}
impl TriggerZoneBundle {
    fn new(trigger_type: TriggerType, size: Vec2, translation: Vec3, color: Color) -> Self {
        Self {
            sprite_bundle: SpriteBundle {
                sprite: Sprite { color, ..default() },
                transform: Transform {
                    translation,
                    scale: size.extend(1.0),
                    rotation: Quat::IDENTITY,
                },
                ..default()
            },
            name: Name::new(format!("Trigger Zone: {}", trigger_type)),
            collider: Collider::cuboid(0.5, 0.5),
            collision_groups: CollisionGroups::new(
                collision_groups::PANEL_TRIGGER_ZONES,
                collision_groups::PANEL_BALLS,
            ),
            trigger_type,
            markers: (ActiveEvents::COLLISION_EVENTS, Sensor),
        }
    }
    // }}}
}
#[derive(Component, Clone, Copy, Default)]
/// Marker to mark this entity as a worker ball.
pub struct WorkerBall;
#[derive(Resource, Clone, Default)]
pub struct WorkerBallSpawner {
    mesh: Mesh2dHandle,
    timer: Timer,
    counter: usize,
}
impl WorkerBallSpawner {
    fn new(mesh: Mesh2dHandle) -> Self {
        let mut timer = Timer::from_seconds(WORKER_BALL_SPAWN_TIMER_SECS, TimerMode::Repeating);
        timer.tick(Duration::from_secs_f32(WORKER_BALL_SPAWN_TIMER_INIT_SECS));
        Self {
            mesh,
            timer,
            counter: 0,
        }
    }
    fn reset(&mut self) {
        self.timer.reset();
        self.timer
            .tick(Duration::from_secs_f32(WORKER_BALL_SPAWN_TIMER_INIT_SECS));
        self.counter = 0;
    }
}
#[derive(Bundle, Clone, Default)]
struct WorkerBallBundle {
    // {{{
    marker: WorkerBall,
    participant: Participant,
    matmesh: MaterialMesh2dBundle<ColorMaterial>,
    collider: Collider,
    collision_groups: CollisionGroups,
    restitution: Restitution,
    rigidbody: RigidBody,
    velocity: Velocity,
    gravity: GravityScale,
    name: Name,
}
impl WorkerBallBundle {
    fn new(
        participant: Participant,
        x: f32,
        mesh: Mesh2dHandle,
        material: Handle<ColorMaterial>,
    ) -> Self {
        Self {
            name: Name::new("Worker Ball"),
            marker: WorkerBall,
            participant,
            matmesh: MaterialMesh2dBundle {
                material,
                mesh,
                transform: Transform::from_xyz(x, WORKER_BALL_SPAWN_Y, WORKER_BALL_Z),
                ..default()
            },
            collider: Collider::ball(WORKER_BALL_RADIUS),
            collision_groups: CollisionGroups::new(
                collision_groups::PANEL_BALLS,
                collision_groups::PANEL_BALLS | PANEL_OBSTACLES | PANEL_TRIGGER_ZONES,
            ),
            restitution: Restitution {
                coefficient: WORKER_BALL_RESTITUTION_COEFFICIENT,
                combine_rule: CoefficientCombineRule::Max,
            },
            rigidbody: RigidBody::Dynamic,
            velocity: Velocity::zero(),
            gravity: GravityScale(WORKER_BALL_GRAVITY_SCALE),
        }
    }
    // }}}
}
#[derive(Component, Clone, Copy)]
pub struct LeftPanelRoot;
#[derive(Component, Clone, Copy)]
pub struct RightPanelRoot;
#[derive(Clone, Bundle)]
/// Component bundle for the round obstacles in the side panels and the walls.
/// (I don't know if meshes and colliders have to be continous. Maybe we can just make a single
/// entity for the entire obstacle course.)
struct ObstacleBundle {
    // {{{
    /// Bevy rendering component used to display the ball.
    matmesh: MaterialMesh2dBundle<ColorMaterial>,
    /// Rapier collider component.
    collider: Collider,
    collision_groups: CollisionGroups,
    /// Rapier rigidbody component. We'll set this to static since we don't want these to move, but
    /// we'd other balls to bounce off it.
    rigidbody: RigidBody,
    name: Name,
}
impl ObstacleBundle {
    fn with_xy(mut self, x: f32, y: f32) -> Self {
        let translation = &mut self.matmesh.transform.translation;
        translation.x = x;
        translation.y = y;
        self
    }
}
fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.insert_resource(WorkerBallSpawner::new(Mesh2dHandle(
        meshes.add(Circle::new(WORKER_BALL_RADIUS)),
    )));
    let left_root = commands
        .spawn((
            Name::new("Left Panel Root"),
            LeftPanelRoot,
            SpatialBundle::from_transform(Transform::from_xyz(LEFT_ROOT_X, 0.0, 0.0)),
            RigidBody::Fixed,
            CollisionGroups::new(
                collision_groups::PANEL_OBSTACLES,
                collision_groups::PANEL_BALLS,
            ),
            Collider::polyline(
                vec![
                    Vec2::new(-ARENA_WIDTH_FRAC_2, ARENA_HEIGHT_FRAC_2),
                    Vec2::new(-ARENA_WIDTH_FRAC_2, -ARENA_HEIGHT_FRAC_2),
                    Vec2::new(ARENA_WIDTH_FRAC_2, -ARENA_HEIGHT_FRAC_2),
                    Vec2::new(ARENA_WIDTH_FRAC_2, ARENA_HEIGHT_FRAC_2),
                    Vec2::new(-ARENA_WIDTH_FRAC_2, ARENA_HEIGHT_FRAC_2),
                ],
                None,
            ),
        ))
        .id();
    let right_root = commands
        .spawn((
            Name::new("Right Panel Root"),
            RightPanelRoot,
            SpatialBundle::from_transform(Transform::from_xyz(RIGHT_ROOT_X, 0.0, 0.0)),
            RigidBody::Fixed,
            CollisionGroups::new(
                collision_groups::PANEL_OBSTACLES,
                collision_groups::PANEL_BALLS,
            ),
            Collider::polyline(
                vec![
                    Vec2::new(-ARENA_WIDTH_FRAC_2, ARENA_HEIGHT_FRAC_2),
                    Vec2::new(-ARENA_WIDTH_FRAC_2, -ARENA_HEIGHT_FRAC_2),
                    Vec2::new(ARENA_WIDTH_FRAC_2, -ARENA_HEIGHT_FRAC_2),
                    Vec2::new(ARENA_WIDTH_FRAC_2, ARENA_HEIGHT_FRAC_2),
                    Vec2::new(-ARENA_WIDTH_FRAC_2, ARENA_HEIGHT_FRAC_2),
                ],
                None,
            ),
        ))
        .id();
    let circle_template = ObstacleBundle {
        matmesh: MaterialMesh2dBundle {
            material: materials.add(CIRCLE_COLOR),
            mesh: Mesh2dHandle(meshes.add(Circle::new(CIRCLE_RADIUS))),
            transform: Transform::from_xyz(0.0, 0.0, CIRCLE_Z),
            ..default()
        },
        collider: Collider::ball(CIRCLE_RADIUS),
        collision_groups: CollisionGroups::new(
            collision_groups::PANEL_OBSTACLES,
            collision_groups::PANEL_BALLS,
        ),
        rigidbody: RigidBody::Fixed,
        name: Name::from("Circle Obstacle"),
    };
    const LENGTH: f32 = TRIGGER_ZONE_DIVIDER_HEIGHT_OFFSET + TRIGGER_ZONE_HEIGHT;
    let divider_template = ObstacleBundle {
        matmesh: MaterialMesh2dBundle {
            material: materials.add(TRIGGER_ZONE_DIVIDER_COLOR),
            mesh: Mesh2dHandle(meshes.add(Capsule2d::new(TRIGGER_ZONE_DIVIDER_RADIUS, LENGTH))),
            transform: Transform::from_xyz(0.0, 0.0, TRIGGER_ZONE_DIVIDER_Z),
            ..default()
        },
        collider: Collider::capsule_y(LENGTH / 2.0, TRIGGER_ZONE_DIVIDER_RADIUS),
        collision_groups: CollisionGroups::new(
            collision_groups::PANEL_OBSTACLES,
            collision_groups::PANEL_BALLS,
        ),
        rigidbody: RigidBody::Fixed,
        name: Name::from("Trigger Zone Divider"),
    };

    let mut f = |root: Entity| {
        for i in 0..CIRCLE_PYRAMID_VERTICAL_COUNT {
            let y = -(i as f32) * (CIRCLE_DIAMETER + CIRCLE_PYRAMID_VERTICAL_GAP)
                + CIRCLE_PYRAMID_VERTICAL_OFFSET;
            if i % 2 == 0 {
                commands
                    .spawn(circle_template.clone().with_xy(0.0, y))
                    .set_parent(root);

                for j in 1..=i / 2 {
                    let x = j as f32 * (CIRCLE_DIAMETER + CIRCLE_PYRAMID_HORIZONTAL_GAP);
                    commands
                        .spawn(circle_template.clone().with_xy(x, y))
                        .set_parent(root);
                    commands
                        .spawn(circle_template.clone().with_xy(-x, y))
                        .set_parent(root);
                }
            } else {
                let x0 = CIRCLE_HALF_GAP + CIRCLE_RADIUS;
                commands
                    .spawn(circle_template.clone().with_xy(x0, y))
                    .set_parent(root);
                commands
                    .spawn(circle_template.clone().with_xy(-x0, y))
                    .set_parent(root);
                for j in 1..(i / 2) + 1 {
                    let x = j as f32 * (CIRCLE_DIAMETER + CIRCLE_PYRAMID_HORIZONTAL_GAP) + x0;
                    commands
                        .spawn(circle_template.clone().with_xy(x, y))
                        .set_parent(root);
                    commands
                        .spawn(circle_template.clone().with_xy(-x, y))
                        .set_parent(root);
                }
            }
        }

        for i in 0..CIRCLE_GRID_VERTICAL_COUNT {
            let y = -(i as f32) * (CIRCLE_DIAMETER + CIRCLE_GRID_VERTICAL_GAP)
                + CIRCLE_GRID_VERTICAL_OFFSET;
            if i % 2 == 0 {
                commands
                    .spawn(circle_template.clone().with_xy(0.0, y))
                    .set_parent(root);

                for j in 1..=CIRCLE_GRID_HORIZONTAL_HALF_COUNT_EVEN_ROW {
                    let x = j as f32 * (CIRCLE_DIAMETER + CIRCLE_GRID_HORIZONTAL_GAP);
                    commands
                        .spawn(circle_template.clone().with_xy(x, y))
                        .set_parent(root);
                    commands
                        .spawn(circle_template.clone().with_xy(-x, y))
                        .set_parent(root);
                }
            } else {
                let x0 = CIRCLE_HALF_GAP + CIRCLE_RADIUS;
                commands
                    .spawn(circle_template.clone().with_xy(x0, y))
                    .set_parent(root);
                commands
                    .spawn(circle_template.clone().with_xy(-x0, y))
                    .set_parent(root);
                for j in 1..CIRCLE_GRID_HORIZONTAL_HALF_COUNT_ODD_ROW {
                    let x = j as f32 * (CIRCLE_DIAMETER + CIRCLE_GRID_HORIZONTAL_GAP) + x0;
                    commands
                        .spawn(circle_template.clone().with_xy(x, y))
                        .set_parent(root);
                    commands
                        .spawn(circle_template.clone().with_xy(-x, y))
                        .set_parent(root);
                }
            }
        }

        commands
            .spawn(
                divider_template
                    .clone()
                    .with_xy(-ARENA_WIDTH_FRAC_10, TRIGGER_ZONE_Y),
            )
            .set_parent(root);
        commands
            .spawn(
                divider_template
                    .clone()
                    .with_xy(-ARENA_WIDTH_FRAC_5 - ARENA_WIDTH_FRAC_10, TRIGGER_ZONE_Y),
            )
            .set_parent(root);
        commands
            .spawn(
                divider_template
                    .clone()
                    .with_xy(ARENA_WIDTH_FRAC_10, TRIGGER_ZONE_Y),
            )
            .set_parent(root);
        commands
            .spawn(
                divider_template
                    .clone()
                    .with_xy(ARENA_WIDTH_FRAC_5 + ARENA_WIDTH_FRAC_10, TRIGGER_ZONE_Y),
            )
            .set_parent(root);
        let mut f = |trigger_type, x, color| {
            commands
                .spawn(TriggerZoneBundle::new(
                    trigger_type,
                    Vec2::new(ARENA_WIDTH_FRAC_5, TRIGGER_ZONE_HEIGHT),
                    Vec3::new(x, TRIGGER_ZONE_Y, TRIGGER_ZONE_Z),
                    color,
                ))
                .set_parent(root);
            commands
                .spawn(Text2dBundle {
                    text: Text::from_section(
                        trigger_type.to_string(),
                        TextStyle {
                            color: TRIGGER_ZONE_TEXT_COLOR,
                            font_size: TRIGGER_ZONE_TEXT_SIZE,
                            ..default()
                        },
                    )
                    .with_justify(JustifyText::Center),
                    transform: Transform {
                        translation: Vec3 {
                            x,
                            y: TRIGGER_ZONE_Y,
                            z: TRIGGER_ZONE_TEXT_OFFSET_Z,
                        },
                        ..default()
                    },
                    ..default()
                })
                .insert(Name::new(format!("Trigger Zone Text: {}", trigger_type)))
                .set_parent(root);
        };
        f(TriggerType::Multiply(4), 0.0, TRIGGER_ZONE_COLOR_0);
        f(
            TriggerType::Multiply(2),
            -ARENA_WIDTH_FRAC_5,
            TRIGGER_ZONE_COLOR_1,
        );
        f(
            TriggerType::Multiply(2),
            ARENA_WIDTH_FRAC_5,
            TRIGGER_ZONE_COLOR_1,
        );
        f(
            TriggerType::BurstShot,
            -2.0 * ARENA_WIDTH_FRAC_5,
            TRIGGER_ZONE_COLOR_2,
        );
        f(
            TriggerType::ChargedShot,
            2.0 * ARENA_WIDTH_FRAC_5,
            TRIGGER_ZONE_COLOR_2,
        );

        commands
            .spawn(SpriteBundle {
                transform: Transform {
                    translation: Vec3::new(0.0, 0.0, WALL_Z),
                    scale: Vec3::new(WALL_WIDTH, WALL_HEIGHT, 1.0),
                    rotation: Quat::IDENTITY,
                },
                sprite: Sprite {
                    color: WALL_COLOR,
                    ..default()
                },
                ..default()
            })
            .insert(Name::new("Panel Wall"))
            .set_parent(root);
        commands
            .spawn(SpriteBundle {
                transform: Transform {
                    translation: Vec3::new(0.0, 0.0, ARENA_Z),
                    scale: Vec3::new(ARENA_WIDTH, ARENA_HEIGHT, 1.0),
                    rotation: Quat::IDENTITY,
                },
                sprite: Sprite {
                    color: ARENA_COLOR,
                    ..default()
                },
                ..default()
            })
            .insert(Name::new("Panel Background"))
            .set_parent(root);
    };
    f(left_root);
    f(right_root);
}
fn spawn_workers_condition(spawner: Res<WorkerBallSpawner>) -> bool {
    spawner.counter < WORKER_BALL_COUNT_MAX
}
pub fn spawn_workers(
    mut commands: Commands,
    mut spawner: ResMut<WorkerBallSpawner>,
    time: Res<Time>,
    rapier: Res<RapierContext>,
    materials: Res<ParticipantMap<Handle<ColorMaterial>>>,
    survivors: Res<ParticipantMap<bool>>,
    left_root: Query<(Entity, &GlobalTransform), With<LeftPanelRoot>>,
    right_root: Query<(Entity, &GlobalTransform), With<RightPanelRoot>>,
) {
    spawner.timer.tick(time.delta());
    if !spawner.timer.just_finished() {
        return;
    }
    let mut f = |a, b, root_entity, root_transform: &GlobalTransform| {
        let collider = Collider::ball(WORKER_BALL_RADIUS);
        let mut caster = WorkerBallShapeCaster::new(
            root_transform.translation().xy(),
            Uniform::new(-ARENA_WIDTH_FRAC_2, ARENA_WIDTH_FRAC_2),
            &rapier,
            &collider,
        );
        match (survivors[a].then_some(a), survivors[b].then_some(b)) {
            (None, None) => (),
            (Some(survivor), None) | (None, Some(survivor)) => {
                let x = caster.get();
                commands
                    .spawn(WorkerBallBundle::new(
                        survivor,
                        x,
                        spawner.mesh.clone(),
                        materials[survivor].clone(),
                    ))
                    .set_parent(root_entity);
            }
            (Some(a), Some(b)) => {
                let mut xa;
                let mut xb;
                loop {
                    xa = caster.get();
                    xb = caster.get();
                    if (xa - xb).abs() > WORKER_BALL_DIAMETER {
                        break;
                    }
                }
                commands
                    .spawn(WorkerBallBundle::new(
                        a,
                        xa,
                        spawner.mesh.clone(),
                        materials[a].clone(),
                    ))
                    .set_parent(root_entity);
                commands
                    .spawn(WorkerBallBundle::new(
                        b,
                        xb,
                        spawner.mesh.clone(),
                        materials[b].clone(),
                    ))
                    .set_parent(root_entity);
            }
        }
    };
    let (left_root_entity, left_root_transform) = left_root.single();
    let (right_root_entity, right_root_transform) = right_root.single();
    f(
        Participant::A,
        Participant::B,
        left_root_entity,
        left_root_transform,
    );
    f(
        Participant::C,
        Participant::D,
        right_root_entity,
        right_root_transform,
    );
    spawner.counter += 1;
}
fn trigger_event(
    mut collision_events: EventReader<CollisionEvent>,
    mut restart_event: EventReader<RestartEvent>,
    mut trigger_event: EventWriter<TriggerEvent>,
    trigger_zone_query: Query<&TriggerType>,
    worker_ball_query: Query<&Participant, With<WorkerBall>>,
) {
    if !restart_event.is_empty() {
        collision_events.clear();
        restart_event.clear();
    }
    for collision_event in collision_events.read() {
        match collision_event {
            &CollisionEvent::Started(a, b, _) => {
                let &trigger_type = if let Ok(x) = trigger_zone_query.get(a) {
                    x
                } else if let Ok(x) = trigger_zone_query.get(b) {
                    x
                } else {
                    continue;
                };
                let &participant = if let Ok(x) = worker_ball_query.get(a) {
                    x
                } else if let Ok(x) = worker_ball_query.get(b) {
                    x
                } else {
                    continue;
                };
                trigger_event.send(TriggerEvent {
                    participant,
                    trigger_type,
                });
            }
            CollisionEvent::Stopped(_, _, _) => (),
        }
    }
}
fn reset_workers(
    mut collision_events: EventReader<CollisionEvent>,
    rapier: Res<RapierContext>,
    left_root: Query<&GlobalTransform, With<LeftPanelRoot>>,
    right_root: Query<&GlobalTransform, With<RightPanelRoot>>,
    trigger_zone_query: Query<(), With<TriggerType>>,
    mut worker_ball_query: Query<
        (&mut Transform, &mut Velocity, &Collider, &Participant),
        With<WorkerBall>,
    >,
) {
    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Started(_, _, _) => (),
            &CollisionEvent::Stopped(a, b, _) => {
                let ball_entity = if trigger_zone_query.get(a).is_ok() {
                    b
                } else if trigger_zone_query.get(b).is_ok() {
                    a
                } else {
                    continue;
                };
                let Ok((mut ball_transform, mut velocity, collider, &participant)) =
                    worker_ball_query.get_mut(ball_entity)
                else {
                    continue;
                };

                let root = match participant {
                    Participant::A | Participant::B => left_root.single(),
                    Participant::C | Participant::D => right_root.single(),
                };
                let x = WorkerBallShapeCaster::new(
                    root.translation().xy(),
                    Uniform::new(-ARENA_WIDTH_FRAC_2, ARENA_WIDTH_FRAC_2),
                    &rapier,
                    collider,
                )
                .get();
                ball_transform.translation.x = x;
                ball_transform.translation.y = WORKER_BALL_SPAWN_Y;
                *velocity = Velocity::zero();
            }
        }
    }
}
struct WorkerBallShapeCaster<'a, 'b, D> {
    root_position: Vec2,
    rng_iter: DistIter<D, ThreadRng, f32>,
    rapier: &'a RapierContext,
    collider: &'b Collider,
}
impl<'a, 'b, D: Distribution<f32>> WorkerBallShapeCaster<'a, 'b, D> {
    fn new(
        root_position: Vec2,
        dist: D,
        rapier: &'a RapierContext,
        collider: &'b Collider,
    ) -> Self {
        Self {
            root_position,
            rng_iter: thread_rng().sample_iter(dist),
            rapier,
            collider,
        }
    }
    fn get(&mut self) -> f32 {
        for x in &mut self.rng_iter {
            if self
                .rapier
                .intersection_with_shape(
                    Vect::new(
                        x + self.root_position.x,
                        WORKER_BALL_SPAWN_Y + self.root_position.y,
                    ),
                    0.0,
                    self.collider,
                    QueryFilter::only_dynamic().groups(CollisionGroups::new(
                        collision_groups::PANEL_BALLS,
                        collision_groups::PANEL_BALLS,
                    )),
                )
                .is_none()
            {
                return x;
            }
        }
        unreachable!("`self.rng_iter: DistIter` is an infinite iterator.");
    }
}
fn restart(
    mut commands: Commands,
    mut spawner: ResMut<WorkerBallSpawner>,
    garbage: Query<Entity, With<WorkerBall>>,
) {
    spawner.reset();
    for entity in garbage.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
