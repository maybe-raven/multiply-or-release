#![allow(dead_code)]

use bevy_rapier2d::geometry::Group;

use crate::participants::Participant;

pub const PANEL_OBSTACLES: Group = Group::GROUP_1;
pub const PANEL_BALLS: Group = Group::GROUP_2;
pub const PANEL_TRIGGER_ZONES: Group = Group::GROUP_3;
pub const TILE_A: Group = Group::GROUP_4;
pub const TILE_B: Group = Group::GROUP_5;
pub const TILE_C: Group = Group::GROUP_6;
pub const TILE_D: Group = Group::GROUP_7;
pub const BULLET_A: Group = Group::GROUP_8;
pub const BULLET_B: Group = Group::GROUP_9;
pub const BULLET_C: Group = Group::GROUP_10;
pub const BULLET_D: Group = Group::GROUP_11;
pub const TURRET_A: Group = Group::GROUP_12;
pub const TURRET_B: Group = Group::GROUP_13;
pub const TURRET_C: Group = Group::GROUP_14;
pub const TURRET_D: Group = Group::GROUP_15;
pub const BATTLEFIELD_ROOT: Group = Group::GROUP_16;
pub const NEW_BULLET_A: Group = Group::GROUP_17;
pub const NEW_BULLET_B: Group = Group::GROUP_18;
pub const NEW_BULLET_C: Group = Group::GROUP_19;
pub const NEW_BULLET_D: Group = Group::GROUP_20;
pub const ALL_TILES: Group =
    Group::from_bits_retain(TILE_A.bits() | TILE_B.bits() | TILE_C.bits() | TILE_D.bits());
pub const ALL_BULLETS: Group =
    Group::from_bits_retain(BULLET_A.bits() | BULLET_B.bits() | BULLET_C.bits() | BULLET_D.bits());
pub const ALL_NEW_BULLETS: Group = Group::from_bits_retain(
    NEW_BULLET_A.bits() | NEW_BULLET_B.bits() | NEW_BULLET_C.bits() | NEW_BULLET_D.bits(),
);
pub const ALL_TURRETS: Group =
    Group::from_bits_retain(TURRET_A.bits() | TURRET_B.bits() | TURRET_C.bits() | TURRET_D.bits());

pub const fn tile(participant: Participant) -> Group {
    match participant {
        Participant::A => TILE_A,
        Participant::B => TILE_B,
        Participant::C => TILE_C,
        Participant::D => TILE_D,
    }
}
pub const fn bullet(participant: Participant) -> Group {
    match participant {
        Participant::A => BULLET_A,
        Participant::B => BULLET_B,
        Participant::C => BULLET_C,
        Participant::D => BULLET_D,
    }
}
pub const fn new_bullet(participant: Participant) -> Group {
    match participant {
        Participant::A => NEW_BULLET_A,
        Participant::B => NEW_BULLET_B,
        Participant::C => NEW_BULLET_C,
        Participant::D => NEW_BULLET_D,
    }
}
pub const fn turret(participant: Participant) -> Group {
    match participant {
        Participant::A => TURRET_A,
        Participant::B => TURRET_B,
        Participant::C => TURRET_C,
        Participant::D => TURRET_D,
    }
}
pub fn all_tiles_except(participant: Participant) -> Group {
    match participant {
        Participant::A => TILE_B | TILE_C | TILE_D,
        Participant::B => TILE_A | TILE_C | TILE_D,
        Participant::C => TILE_A | TILE_B | TILE_D,
        Participant::D => TILE_A | TILE_B | TILE_C,
    }
}
pub fn all_bullets_except(participant: Participant) -> Group {
    match participant {
        Participant::A => BULLET_B | BULLET_C | BULLET_D,
        Participant::B => BULLET_A | BULLET_C | BULLET_D,
        Participant::C => BULLET_A | BULLET_B | BULLET_D,
        Participant::D => BULLET_A | BULLET_B | BULLET_C,
    }
}
pub fn all_new_bullets_except(participant: Participant) -> Group {
    match participant {
        Participant::A => NEW_BULLET_B | NEW_BULLET_C | NEW_BULLET_D,
        Participant::B => NEW_BULLET_A | NEW_BULLET_C | NEW_BULLET_D,
        Participant::C => NEW_BULLET_A | NEW_BULLET_B | NEW_BULLET_D,
        Participant::D => NEW_BULLET_A | NEW_BULLET_B | NEW_BULLET_C,
    }
}
pub fn all_turrets_except(participant: Participant) -> Group {
    match participant {
        Participant::A => TURRET_B | TURRET_C | TURRET_D,
        Participant::B => TURRET_A | TURRET_C | TURRET_D,
        Participant::C => TURRET_A | TURRET_B | TURRET_D,
        Participant::D => TURRET_A | TURRET_B | TURRET_C,
    }
}
