use bevy::{color::palettes::css, prelude::*};
use std::ops::{Index, IndexMut};

// Constants {{{

const PARTICIPANT_COLORS: ParticipantMap<Srgba> = ParticipantMap::new(
    css::MAROON,
    css::DARK_GREEN,
    css::PURPLE,
    css::DARK_GOLDENROD,
);
const BALL_COLORS: ParticipantMap<Srgba> =
    ParticipantMap::new(css::RED, css::LIMEGREEN, css::VIOLET, css::YELLOW);

// }}}

pub struct UtilsPlugin;
impl Plugin for UtilsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, (setup_participant_maps,));
    }
}

#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct TileColor(pub Color);
#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct BallColor(pub Color);

/// A struct that maps a value to each participant.
#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct ParticipantMap<T> {
    // {{{
    pub a: T,
    pub b: T,
    pub c: T,
    pub d: T,
}
impl<T> ParticipantMap<T> {
    pub const fn new(a: T, b: T, c: T, d: T) -> Self {
        Self { a, b, c, d }
    }
    pub const fn get(&self, participant: Participant) -> &T {
        match participant {
            Participant::A => &self.a,
            Participant::B => &self.b,
            Participant::C => &self.c,
            Participant::D => &self.d,
        }
    }
    pub fn set(&mut self, participant: Participant, new_value: T) {
        match participant {
            Participant::A => self.a = new_value,
            Participant::B => self.b = new_value,
            Participant::C => self.c = new_value,
            Participant::D => self.d = new_value,
        }
    }
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> ParticipantMap<U> {
        ParticipantMap::new(f(self.a), f(self.b), f(self.c), f(self.d))
    }
    // }}}
}
impl<T> Index<Participant> for ParticipantMap<T> {
    type Output = T;
    fn index(&self, index: Participant) -> &Self::Output {
        match index {
            Participant::A => &self.a,
            Participant::B => &self.b,
            Participant::C => &self.c,
            Participant::D => &self.d,
        }
    }
}
impl<T> IndexMut<Participant> for ParticipantMap<T> {
    fn index_mut(&mut self, index: Participant) -> &mut Self::Output {
        match index {
            Participant::A => &mut self.a,
            Participant::B => &mut self.b,
            Participant::C => &mut self.c,
            Participant::D => &mut self.d,
        }
    }
}
impl<T: Copy> ParticipantMap<T> {
    pub const fn splat(x: T) -> Self {
        Self {
            a: x,
            b: x,
            c: x,
            d: x,
        }
    }
}

#[derive(Debug, Component, Clone, Copy, Default, PartialEq, Eq)]
/// A game participant. It's not called player since the game is not interactive.
pub enum Participant {
    #[default]
    A,
    B,
    C,
    D,
}
impl Participant {
    pub const ALL: [Self; 4] = [Self::A, Self::B, Self::C, Self::D];
}
impl std::fmt::Display for Participant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Participant::A => "RED",
            Participant::B => "GREEN",
            Participant::C => "VIOLET",
            Participant::D => "YELLOW",
        };
        f.write_str(name)
    }
}
pub fn setup_participant_maps(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(ParticipantMap::splat(true));
    commands.insert_resource(PARTICIPANT_COLORS.map(Color::Srgba).map(TileColor));
    commands.insert_resource(BALL_COLORS.map(Color::Srgba).map(BallColor));
    commands.insert_resource(
        BALL_COLORS.map(|srgba| materials.add(ColorMaterial::from(Color::from(srgba)))),
    );
}
