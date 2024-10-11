#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use crate::{
    battlefield::{RestartEvent, TileHitEvent},
    panel_plugin::{
        spawn_workers, WorkerBall, LEFT_ROOT_X, RIGHT_ROOT_X, WORKER_BALL_COUNT_MAX,
        WORKER_BALL_RADIUS, WORKER_BALL_SPAWN_Y,
    },
    participants::{self, Participant, BALL_COLORS, TILE_COLORS},
};
use bevy::prelude::*;
use bevy_hanabi::prelude::*;

// Constants {{{

const HIT_PARTICLE_BUFFER_SIZE: u32 = 65536;
const HIT_PARTICLE_SIZE: f32 = 1.25;
const HIT_PARTICLE_LIFETIME: f32 = 1.5;
const HIT_PARTICLE_COUNT: f32 = 12.0;
pub const HIT_PARTICLE_MAX_PER_SECOND: f32 =
    HIT_PARTICLE_BUFFER_SIZE as f32 / HIT_PARTICLE_LIFETIME / HIT_PARTICLE_COUNT;
const TRAIL_PARTICLE_SIZE: f32 = WORKER_BALL_RADIUS * 2.0;
const TRAIL_SPAWN_RATE: f32 = 60.;
pub const TRAIL_LIFETIME: f32 = 0.5;
pub const SPAWN_COLOR_PROPERTY: &str = "spawn_color";
const POSITION_PROPERTY: &str = "position";
const BULLET_VEL_PROPERTY: &str = "bullet_vel";

// }}}

pub struct EffectsPlugin;
impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HanabiPlugin)
            .add_systems(
                PreStartup,
                (setup_tile_hit_effect, setup_trail_effect).after(participants::setup),
            )
            .add_systems(
                Update,
                (
                    (update_worker_trail_position, add_worker_trails)
                        .chain()
                        .before(spawn_workers),
                    trigger_tile_hit_effect
                        .run_if(on_event::<TileHitEvent>().or_else(on_event::<RestartEvent>())),
                ),
            )
            .add_systems(PostUpdate, restart.run_if(on_event::<RestartEvent>()));
    }
}
#[derive(Clone, Resource)]
struct TileHitEffect(pub Handle<EffectAsset>);
#[derive(Clone, Resource)]
struct TrailEffect(pub Handle<EffectAsset>);
#[derive(Component, Clone, Copy)]
struct TileHitEffectSpawner;
#[derive(Component, Clone, Copy)]
struct WorkerBallTrail(pub Entity);
#[derive(Component, Clone, Copy)]
struct InactiveLeftWorkerBallTrail;
#[derive(Component, Clone, Copy)]
struct InactiveRightWorkerBallTrail;
#[derive(Bundle, Clone)]
struct WorkerBallTrailBundle {
    // {{{
    link: WorkerBallTrail,
    peb: ParticleEffectBundle,
    name: Name,
}
impl WorkerBallTrailBundle {
    fn new(
        target: Entity,
        position: Vec3,
        color: impl Into<LinearRgba>,
        effect: Handle<EffectAsset>,
    ) -> Self {
        Self {
            link: WorkerBallTrail(target),
            peb: ParticleEffectBundle {
                effect: ParticleEffect::new(effect),
                effect_properties: EffectProperties::from_spawn_color(color)
                    .with_position(position),
                ..default()
            },
            name: Name::new("Worker Ball Trail"),
        }
    }
}
fn setup_tile_hit_effect(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    // Set `spawn_immediately` to false to spawn on command with Spawner::reset()
    let spawner = Spawner::once(HIT_PARTICLE_COUNT.into(), true);

    let writer = ExprWriter::new();

    // Init the age of particles to 0, and their lifetime to 1.5 second.
    let age = writer.lit(0.);
    let init_age = SetAttributeModifier::new(Attribute::AGE, age.expr());
    // Initialize the total lifetime of the particle, that is
    // the time for which it's simulated and rendered. This modifier
    // is almost always required, otherwise the particles won't show.
    let lifetime = writer.lit(HIT_PARTICLE_LIFETIME);
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime.expr());

    // Add a bit of linear drag to slow down particles after the inital spawning.
    // This keeps the particle around the spawn point, making it easier to visualize
    // the different groups of particles.
    let drag = writer.lit(3.);
    let update_drag = LinearDragModifier::new(drag.expr());

    // Bind the initial particle color to the value of the 'spawn_color' property
    // when the particle spawns. The particle will keep that color afterward,
    // even if the property changes, because the color will be saved
    // per-particle (due to the Attribute::COLOR).
    let spawn_color = writer.add_property(SPAWN_COLOR_PROPERTY, 0xFFFFFFFFu32.into());
    let init_color = SetAttributeModifier::new(Attribute::COLOR, writer.prop(spawn_color).expr());

    let gradient = Gradient::linear(Vec2::splat(HIT_PARTICLE_SIZE), Vec2::ZERO);

    // On spawn, randomly initialize the position of the particle
    // to be over the surface of a sphere of radius 2 units.
    let init_pos = SetPositionCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Z).expr(),
        radius: writer.lit(2.).expr(),
        dimension: ShapeDimension::Volume,
    };

    let bullet_vel_prop = writer.add_property(BULLET_VEL_PROPERTY, Vec2::ZERO.into());
    let bullet_vel = writer.prop(bullet_vel_prop);
    let bullet_vel3 = bullet_vel.clone().x().vec3(bullet_vel.y(), writer.lit(0.));
    let vel = writer
        .attr(Attribute::POSITION)
        .normalized()
        .mul(writer.lit(7.5).uniform(writer.lit(10.)))
        .add(
            bullet_vel3
                .normalized()
                .mul(writer.lit(10.).uniform(writer.lit(15.))),
        );
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, vel.expr());

    let effect = effects.add(
        EffectAsset::new(vec![HIT_PARTICLE_BUFFER_SIZE], spawner, writer.finish())
            .with_name("tile hit")
            .init(init_pos)
            .init(init_vel)
            .init(init_age)
            .init(init_lifetime)
            .init(init_color)
            .update(update_drag)
            .render(SizeOverLifetimeModifier {
                gradient,
                screen_space_size: false,
            }),
    );

    commands.insert_resource(TileHitEffect(effect));
}
fn setup_trail_effect(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    let writer = ExprWriter::default();

    let pos = writer.add_property(POSITION_PROPERTY, Vec3::ZERO.into());
    let spawn_color = writer.add_property(SPAWN_COLOR_PROPERTY, 0xFFFFFFFFu32.into());

    let init_position_attr = SetAttributeModifier {
        attribute: Attribute::POSITION,
        value: writer.prop(pos).expr(),
    };

    let init_velocity_attr = SetAttributeModifier {
        attribute: Attribute::VELOCITY,
        value: writer.lit(Vec3::ZERO).expr(),
    };

    let init_age_attr = SetAttributeModifier {
        attribute: Attribute::AGE,
        value: writer.lit(0.0).expr(),
    };

    let init_lifetime_attr = SetAttributeModifier {
        attribute: Attribute::LIFETIME,
        value: writer.lit(999999.0).expr(),
    };

    let init_size_attr = SetAttributeModifier {
        attribute: Attribute::SIZE,
        value: writer.lit(TRAIL_PARTICLE_SIZE).expr(),
    };

    let init_color = SetAttributeModifier {
        attribute: Attribute::COLOR,
        value: writer.lit(LinearRgba::NONE.as_u32()).expr(),
    };

    let clone1_modifier = CloneModifier::new(1.0 / TRAIL_SPAWN_RATE, 1);
    let clone2_modifier = CloneModifier::new(1.0 / TRAIL_SPAWN_RATE, 2);

    let move_modifier = SetAttributeModifier {
        attribute: Attribute::POSITION,
        value: writer.prop(pos).expr(),
    };

    let update_lifetime_attr = SetAttributeModifier {
        attribute: Attribute::LIFETIME,
        value: writer.lit(TRAIL_LIFETIME).expr(),
    };

    let age_ratio = writer
        .attr(Attribute::AGE)
        .div(writer.attr(Attribute::LIFETIME));
    let size = writer
        .lit(TRAIL_PARTICLE_SIZE)
        .mix(writer.lit(0.0), age_ratio.clone());
    let update_size_attr = SetAttributeModifier {
        attribute: Attribute::SIZE,
        value: size.expr(),
    };

    let alpha_offset = age_ratio
        .smoothstep(
            writer.lit(0.0),
            writer.prop(spawn_color).unpack4x8unorm().w(),
        )
        .mul(writer.lit(Vec4::new(0.0, 0.0, 0.0, 1.0)))
        .pack4x8unorm();
    let color = writer.prop(spawn_color).sub(alpha_offset);
    let update_color_attr = SetAttributeModifier {
        attribute: Attribute::COLOR,
        value: color.expr(),
    };
    let round = RoundModifier {
        roundness: writer.lit(1.0).expr(),
    };

    let group0 = ParticleGroupSet::single(0);
    let group12 = ParticleGroupSet::single(1).with_group(2);
    const TOTAL_BALL_COUNT: u32 = WORKER_BALL_COUNT_MAX as u32 * 4;
    const PARTICLE_COUNT: u32 =
        (TOTAL_BALL_COUNT as f32 * TRAIL_SPAWN_RATE * TRAIL_LIFETIME + 1.0) as u32;
    let effect = EffectAsset::new(
        vec![TOTAL_BALL_COUNT, PARTICLE_COUNT, PARTICLE_COUNT],
        Spawner::once(1.0.into(), true),
        writer.finish(),
    )
    .with_name("trail")
    .with_simulation_space(SimulationSpace::Global)
    .init(init_position_attr)
    .init(init_velocity_attr)
    .init(init_age_attr)
    .init(init_lifetime_attr)
    .init(init_size_attr)
    .init(init_color)
    .update_groups(move_modifier, group0)
    .update_groups(clone1_modifier, group0)
    .update_groups(clone2_modifier, group0)
    .update_groups(update_lifetime_attr, group12)
    .update_groups(update_color_attr, group12)
    .update_groups(update_size_attr, group12)
    .render_groups(round, group0.with_group(1))
    .render_groups(RibbonModifier, ParticleGroupSet::single(2));

    commands.insert_resource(TrailEffect(effects.add(effect)));
}
fn add_worker_trails(
    mut commands: Commands,
    mut left_trails: Query<
        (Entity, &mut EffectProperties),
        (
            With<InactiveLeftWorkerBallTrail>,
            Without<InactiveRightWorkerBallTrail>,
        ),
    >,
    mut right_trails: Query<
        (Entity, &mut EffectProperties),
        (
            With<InactiveRightWorkerBallTrail>,
            Without<InactiveLeftWorkerBallTrail>,
        ),
    >,
    workers: Query<(Entity, &GlobalTransform, &Participant), Added<WorkerBall>>,
    effect: Res<TrailEffect>,
) {
    let mut left_trails = left_trails.iter_mut();
    let mut right_trails = right_trails.iter_mut();
    for (worker_entity, transform, &participant) in workers.iter() {
        if let Some((trail_entity, mut trail_properties)) = match participant {
            Participant::A | Participant::B => left_trails.next(),
            Participant::C | Participant::D => right_trails.next(),
        } {
            commands
                .entity(trail_entity)
                .remove::<(InactiveLeftWorkerBallTrail, InactiveRightWorkerBallTrail)>()
                .insert(WorkerBallTrail(worker_entity));
            trail_properties.set_spawn_color(TILE_COLORS[participant]);
            trail_properties.set_position(transform.translation());
        } else {
            commands.spawn(WorkerBallTrailBundle::new(
                worker_entity,
                transform.translation(),
                TILE_COLORS[participant],
                effect.0.clone(),
            ));
        }
    }
}
fn update_worker_trail_position(
    mut commands: Commands,
    mut query: Query<((Entity, &WorkerBallTrail), &mut EffectProperties)>,
    transform_query: Query<&GlobalTransform>,
    mut go_left: Local<bool>,
) {
    for ((trail_entity, &WorkerBallTrail(ball_entity)), mut properties) in &mut query {
        if let Ok(transform) = transform_query.get(ball_entity) {
            properties.set_position(transform.translation());
        } else {
            // Despawning the particle effect causes immense lag for some reason,
            // so instead we just leave it running but make it invisible
            let mut trail = commands.entity(trail_entity);
            trail.remove::<WorkerBallTrail>();
            let x = if *go_left {
                trail.insert(InactiveLeftWorkerBallTrail);
                LEFT_ROOT_X
            } else {
                trail.insert(InactiveRightWorkerBallTrail);
                RIGHT_ROOT_X
            };
            properties.set_spawn_color(LinearRgba::NONE);
            properties.set_position(Vec3::new(x, WORKER_BALL_SPAWN_Y, 0.0));
            *go_left = !*go_left;
        }
    }
}
fn restart(
    mut commands: Commands,
    mut trails: Query<
        (Entity, &mut EffectProperties),
        Or<(
            With<InactiveLeftWorkerBallTrail>,
            With<InactiveRightWorkerBallTrail>,
            With<WorkerBallTrail>,
        )>,
    >,
) {
    let mut go_left = false;
    for (entity, mut properties) in trails.iter_mut() {
        let x = if go_left { LEFT_ROOT_X } else { RIGHT_ROOT_X };
        properties.set_spawn_color(LinearRgba::NONE);
        properties.set_position(Vec3::new(x, WORKER_BALL_SPAWN_Y, 0.0));
        let mut trail = commands.entity(entity);
        trail.remove::<(
            InactiveLeftWorkerBallTrail,
            InactiveRightWorkerBallTrail,
            WorkerBallTrail,
        )>();
        if go_left {
            trail.insert(InactiveLeftWorkerBallTrail);
        } else {
            trail.insert(InactiveRightWorkerBallTrail);
        }
        go_left = !go_left;
    }
}
fn trigger_tile_hit_effect(
    mut commands: Commands,
    mut tile_hit_events: EventReader<TileHitEvent>,
    mut restart_event: EventReader<RestartEvent>,
    mut effect_spawners: Query<
        (&mut Transform, &mut EffectProperties, &mut EffectSpawner),
        With<TileHitEffectSpawner>,
    >,
    effect: Res<TileHitEffect>,
    time: Res<Time>,
) {
    if !restart_event.is_empty() {
        tile_hit_events.clear();
        restart_event.clear();
    }
    let mut spawners = effect_spawners.iter_mut();
    // Maximum number of spawners left available this frame for tile hit particle effect.
    let mut count = (time.delta_seconds() * HIT_PARTICLE_MAX_PER_SECOND).floor() as usize;
    for tile_hit in tile_hit_events.read() {
        if count == 0 {
            return;
        }
        count -= 1;
        if let Some((mut spawner_transform, mut properties, mut spawner)) = spawners.next() {
            spawner_transform.translation = tile_hit.position;
            properties.set_spawn_color(BALL_COLORS[tile_hit.participant]);
            properties.set_bullet_vel(tile_hit.bullet_velocity);
            spawner.reset();
        } else {
            commands.spawn((
                TileHitEffectSpawner,
                Name::new("Tile Hit Particle Spawner"),
                ParticleEffectBundle {
                    transform: Transform::from_translation(tile_hit.position),
                    effect: ParticleEffect::new(effect.0.clone()),
                    effect_properties: EffectProperties::from_spawn_color(
                        BALL_COLORS[tile_hit.participant],
                    ),
                    ..default()
                },
            ));
        }
    }
}
pub trait EffectPropertiesExt: Default {
    fn set_spawn_color(&mut self, color: impl Into<LinearRgba>);
    fn set_bullet_vel(&mut self, bullet_vel: Vec2);
    fn set_position(&mut self, position: Vec3);
    fn from_spawn_color(color: impl Into<LinearRgba>) -> Self {
        let mut properties = Self::default();
        properties.set_spawn_color(color);
        properties
    }
    fn with_position(mut self, position: Vec3) -> Self {
        self.set_position(position);
        self
    }
}
impl EffectPropertiesExt for EffectProperties {
    fn set_spawn_color(&mut self, color: impl Into<LinearRgba>) {
        self.set("spawn_color", color.into().as_u32().into());
    }
    fn set_bullet_vel(&mut self, bullet_vel: Vec2) {
        self.set("bullet_vel", bullet_vel.into());
    }
    fn set_position(&mut self, position: Vec3) {
        self.set("position", position.into());
    }
}
