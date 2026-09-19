use crate::data::GameData;
use crate::player::{Player, PlayerState};
use crate::settings::GameSettings;
use crate::ui::Paused;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

pub fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<GameData>,
    mut player_state: Single<&mut PlayerState, With<Player>>,
) {
    const DIGIT_KEYS: [KeyCode; 9] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    let mut move_input = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyD) {
        move_input.x += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        move_input.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyW) {
        move_input.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        move_input.y -= 1.0;
    }
    let move_input = move_input.normalize_or_zero();

    player_state.move_input = move_input;
    player_state.jump_held = keys.pressed(KeyCode::Space);
    player_state.crouching = keys.pressed(KeyCode::ControlLeft);
    player_state.sprinting =
        keys.pressed(KeyCode::ShiftLeft) && !player_state.crouching && move_input.y > 0.0;

    for block in data.blocks.iter().enumerate() {
        if let Some(key) = block.1.hotbar_key
            && keys.just_pressed(DIGIT_KEYS[key as usize - 1])
        {
            player_state.selected_block = block.0 as u8;
        }
    }
}

pub fn mouse_look(
    mouse_motion: Res<AccumulatedMouseMotion>,
    settings: Res<GameSettings>,
    data: Res<GameData>,
    mut player: Single<(&mut PlayerState, &mut Transform), With<Player>>,
) {
    let (player_state, player_transform) = &mut *player;
    let sensitivity = data.camera.look_sensitivity * settings.mouse_sensitivity;

    player_state.yaw -= mouse_motion.delta.x * sensitivity;
    player_state.pitch = (player_state.pitch - mouse_motion.delta.y * sensitivity)
        .clamp(-data.camera.max_pitch, data.camera.max_pitch);

    player_transform.rotation = Quat::from_rotation_y(player_state.yaw);
}

pub fn sync_camera(
    player: Single<(&Transform, &PlayerState), With<Player>>,
    mut camera_transform: Single<&mut Transform, (With<Camera3d>, Without<Player>)>,
    paused: Res<Paused>,
    settings: Res<GameSettings>,
    data: Res<GameData>,
    fixed_time: Res<Time<Fixed>>,
    mut camera_projection: Single<&mut Projection, (With<Camera3d>, Without<Player>)>,
) {
    let (player_transform, player_state) = *player;
    let eye_height = if player_state.crouching {
        data.player.crouch_eye_height
    } else {
        data.player.eye_height
    };

    let render_position = if paused.0 {
        player_transform.translation
    } else {
        player_state
            .prev_position
            .lerp(player_transform.translation, fixed_time.overstep_fraction())
    };

    camera_transform.translation = render_position + Vec3::Y * eye_height;
    camera_transform.rotation =
        Quat::from_rotation_y(player_state.yaw) * Quat::from_rotation_x(player_state.pitch);

    if settings.is_changed()
        && let Projection::Perspective(perspective) = &mut **camera_projection
    {
        perspective.fov = settings.fov.to_radians();
    }
}
