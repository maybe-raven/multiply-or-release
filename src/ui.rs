#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use crate::{
    battlefield::{game_is_going, EliminationEvent, RestartEvent},
    participants::BALL_COLORS,
};
use bevy::prelude::*;

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                button_system.run_if(not(game_is_going)),
                restart.run_if(on_event::<RestartEvent>()),
                add_elimination_text.run_if(on_event::<EliminationEvent>()),
                remove_elimination_text.run_if(any_with_component::<EliminationTextTimer>),
                add_game_over_text.run_if(not(game_is_going)),
            ),
        );
    }
}

// CONSTANTS {{{

const ELIMINATION_TEXT_DURATION: f32 = 4.0;
const ELIMINATION_TEXT_FONT_SIZE: f32 = 48.0;
const GAME_OVER_TEXT_FONT_SIZE: f32 = 72.0;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);

// }}}

#[derive(Clone, Copy, Component)]
struct UIRoot;
#[derive(Clone, Copy, Component)]
struct RestartButton;
#[derive(Component)]
struct EliminationTextTimer(Timer);
#[derive(Bundle)]
struct EliminationTextBundle {
    text_bundle: TextBundle,
    timer: EliminationTextTimer,
}
impl EliminationTextBundle {
    fn new(participant: impl std::fmt::Display, color: impl Into<Color>) -> Self {
        EliminationTextBundle {
            text_bundle: TextBundle::from_section(
                format!("{} Eliminated", participant),
                TextStyle {
                    font: default(),
                    font_size: ELIMINATION_TEXT_FONT_SIZE,
                    color: color.into(),
                },
            ),
            timer: EliminationTextTimer(Timer::from_seconds(
                ELIMINATION_TEXT_DURATION,
                TimerMode::Once,
            )),
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        UIRoot,
        NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::top(Val::Percent(10.0)),
                ..default()
            },
            // transform: Transform::from_xyz(0.0, 0.0, ELIMINATION_TEXT_Z),
            ..default()
        },
    ));
    let button = commands
        .spawn((
            RestartButton,
            ButtonBundle {
                style: Style {
                    width: Val::Px(200.0),
                    height: Val::Px(65.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_self: JustifySelf::Center,
                    align_self: AlignSelf::Center,
                    // horizontally center child text
                    justify_content: JustifyContent::Center,
                    // vertically center child text
                    align_items: AlignItems::Center,
                    ..default()
                },
                visibility: Visibility::Hidden,
                border_color: BorderColor(Color::BLACK),
                border_radius: BorderRadius::MAX,
                background_color: NORMAL_BUTTON.into(),
                ..default()
            },
        ))
        .id();
    commands
        .spawn(TextBundle::from_section(
            "Restart",
            TextStyle {
                font: default(),
                font_size: 40.0,
                color: Color::srgb(0.9, 0.9, 0.9),
            },
        ))
        .set_parent(button);
}
fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut events: EventWriter<RestartEvent>,
) {
    for (interaction, mut color, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                events.send_default();
                // *color = PRESSED_BUTTON.into();
                // border_color.0 = RED.into();
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}
fn add_elimination_text(
    mut commands: Commands,
    mut events: EventReader<EliminationEvent>,
    ui_root: Query<Entity, With<UIRoot>>,
) {
    for event in events.read() {
        commands
            .spawn(EliminationTextBundle::new(
                event.participant,
                BALL_COLORS[event.participant],
            ))
            .set_parent(ui_root.single());
    }
}
fn remove_elimination_text(
    mut commands: Commands,
    mut query: Query<(Entity, &mut EliminationTextTimer)>,
    time: Res<Time>,
) {
    for (text_id, mut timer) in &mut query {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            commands.entity(text_id).despawn_recursive();
        }
    }
}
fn add_game_over_text(
    mut commands: Commands,
    ui_root: Query<Entity, With<UIRoot>>,
    mut restart_button: Query<&mut Visibility, With<RestartButton>>,
) {
    if restart_button.single() == Visibility::Visible {
        return;
    }
    *restart_button.single_mut() = Visibility::Visible;
    let text_id = commands
        .spawn(TextBundle::from_section(
            "Game Over",
            TextStyle {
                font: default(),
                font_size: GAME_OVER_TEXT_FONT_SIZE,
                color: Color::BLACK,
            },
        ))
        .id();
    commands
        .entity(ui_root.single())
        .insert_children(0, &[text_id]);
}
fn restart(
    mut commands: Commands,
    query: Query<&Children, With<UIRoot>>,
    mut button_visibility: Query<&mut Visibility, With<RestartButton>>,
) {
    for &child in query.single().iter() {
        commands.entity(child).despawn_recursive();
        *button_visibility.single_mut() = Visibility::Hidden;
    }
}
