#![allow(dead_code)]

use crate::{
    battlefield::EliminationEvent,
    panel_plugin::{TriggerEvent, TriggerType},
    participants::Participant,
};
use bevy::prelude::*;

pub struct DebugUtilsPlugin;
impl Plugin for DebugUtilsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
            // app.add_plugins(bevy_rapier2d::render::RapierDebugRenderPlugin::default())
            .insert_resource(AutoTimer::default())
            .add_systems(Update, auto_elimination);
    }
}

#[derive(Resource, Deref, DerefMut)]
struct AutoTimer(Timer);
impl Default for AutoTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(10.0, TimerMode::Once))
    }
}
fn auto_elimination(
    mut writer: EventWriter<EliminationEvent>,
    mut timer: ResMut<AutoTimer>,
    time: Res<Time>,
) {
    timer.tick(time.delta());
    if timer.just_finished() {
        writer.send(EliminationEvent {
            participant: Participant::A,
        });
        writer.send(EliminationEvent {
            participant: Participant::B,
        });
        writer.send(EliminationEvent {
            participant: Participant::C,
        });
    }
}
fn auto_fire(mut writer: EventWriter<TriggerEvent>, mut timer: ResMut<AutoTimer>, time: Res<Time>) {
    timer.tick(time.delta());
    if timer.just_finished() {
        for participant in Participant::ALL {
            for _ in 0..3 {
                writer.send(TriggerEvent {
                    participant,
                    trigger_type: TriggerType::Multiply(4),
                });
            }
            writer.send(TriggerEvent {
                participant,
                trigger_type: TriggerType::BurstShot,
            });
        }
    }
}
fn auto_multiply(
    mut writer: EventWriter<TriggerEvent>,
    mut timer: ResMut<AutoTimer>,
    time: Res<Time>,
) {
    timer.tick(time.delta());
    if timer.just_finished() {
        writer.send(TriggerEvent {
            participant: Participant::A,
            trigger_type: TriggerType::Multiply(4),
        });
    }
}
fn print_trigger_events(mut events: EventReader<TriggerEvent>) {
    for event in events.read() {
        println!("{:#?}", event);
    }
}
