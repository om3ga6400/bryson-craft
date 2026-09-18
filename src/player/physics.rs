use crate::data::GameData;
use crate::player::{Player, PlayerState};
use crate::world::GameWorld;
use bevy::prelude::*;
use bevy_voxel_world::prelude::{VoxelWorld, WorldVoxel};

pub fn step_player_physics(
    time: Res<Time>,
    mut player: Single<(&mut Transform, &mut PlayerState), With<Player>>,
    voxel_world: VoxelWorld<GameWorld>,
    data: Res<GameData>,
) {
    let (player_transform, player_state) = &mut *player;
    let delta_time = time.delta_secs();

    player_state.prev_position = player_transform.translation;

    let half_extents = if player_state.crouching {
        data.player.crouching_extents()
    } else {
        data.player.standing_extents()
    };
    let move_speed = if player_state.crouching {
        data.player.crouch_speed
    } else if player_state.sprinting {
        data.player.sprint_speed
    } else {
        data.player.walk_speed
    };

    let yaw_rotation = Quat::from_rotation_y(player_state.yaw);
    let move_direction = yaw_rotation * Vec3::X * player_state.move_input.x
        + yaw_rotation * Vec3::NEG_Z * player_state.move_input.y;

    player_state.velocity.x = move_direction.x * move_speed;
    player_state.velocity.z = move_direction.z * move_speed;

    if player_state.on_ground && player_state.jump_held {
        player_state.velocity.y = data.player.jump_speed;
        player_state.on_ground = false;
    }

    player_state.velocity.y = (player_state.velocity.y + data.player.gravity * delta_time)
        .max(data.player.terminal_velocity);

    let mut position = player_transform.translation;

    move_along_axis(
        &mut position,
        &mut player_state.velocity.x,
        Vec3::X,
        delta_time,
        false,
        &voxel_world,
        half_extents,
        data.player.collision_step,
    );
    move_along_axis(
        &mut position,
        &mut player_state.velocity.z,
        Vec3::Z,
        delta_time,
        false,
        &voxel_world,
        half_extents,
        data.player.collision_step,
    );

    player_state.on_ground = move_along_axis(
        &mut position,
        &mut player_state.velocity.y,
        Vec3::Y,
        delta_time,
        true,
        &voxel_world,
        half_extents,
        data.player.collision_step,
    );

    player_transform.translation = position;
}

fn move_along_axis(
    position: &mut Vec3,
    axis_velocity: &mut f32,
    axis: Vec3,
    delta_time: f32,
    is_vertical: bool,
    voxel_world: &VoxelWorld<GameWorld>,
    half_extents: Vec3,
    collision_step: f32,
) -> bool {
    let frame_movement = *axis_velocity * delta_time;
    if frame_movement == 0.0 {
        return false;
    }

    let step_count = (frame_movement.abs() / collision_step).ceil() as u32;
    let step_size = frame_movement / step_count as f32;

    let mut landed = false;

    for _ in 0..step_count {
        let previous_position = *position;
        *position += axis * step_size;

        if intersects_solid(voxel_world, *position, half_extents) {
            *position = previous_position;
            landed = is_vertical && *axis_velocity < 0.0;
            *axis_velocity = 0.0;
            break;
        }
    }

    landed
}

fn intersects_solid(voxel_world: &VoxelWorld<GameWorld>, center: Vec3, half_extents: Vec3) -> bool {
    let player_min = center - half_extents;
    let player_max = center + half_extents;

    let min = player_min.floor().as_ivec3();
    let max = player_max.floor().as_ivec3();

    for x in min.x..=max.x {
        for y in min.y..=max.y {
            for z in min.z..=max.z {
                let voxel_pos = IVec3::new(x, y, z);
                if matches!(voxel_world.get_voxel(voxel_pos), WorldVoxel::Solid(_)) {
                    let block_min = voxel_pos.as_vec3();
                    let block_max = block_min + Vec3::ONE;
                    if player_min.x < block_max.x
                        && player_max.x > block_min.x
                        && player_min.y < block_max.y
                        && player_max.y > block_min.y
                        && player_min.z < block_max.z
                        && player_max.z > block_min.z
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}
