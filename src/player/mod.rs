pub mod input;
pub mod interaction;
pub mod physics;

use bevy::prelude::*;

#[derive(Component, Default)]
pub struct Player;

#[derive(Component, Default)]
pub struct PlayerState {
    pub velocity: Vec3,
    pub on_ground: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub move_input: Vec2,
    pub jump_held: bool,
    pub sprinting: bool,
    pub crouching: bool,
    pub selected_block: u8,
    pub prev_position: Vec3,
}
