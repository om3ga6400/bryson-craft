use crate::ui::Paused;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

pub fn grab_cursor(mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    lock_cursor(&mut cursor_options);
}

pub fn release_cursor(cursor_options: &mut CursorOptions) {
    cursor_options.visible = true;
    cursor_options.grab_mode = CursorGrabMode::None;
}

pub fn lock_cursor(cursor_options: &mut CursorOptions) {
    cursor_options.visible = false;
    cursor_options.grab_mode = CursorGrabMode::Locked;
}

pub fn manage_cursor(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    paused: Res<Paused>,
) {
    if paused.0 {
        return;
    }

    if mouse_buttons.just_pressed(MouseButton::Left) {
        lock_cursor(&mut cursor_options);
    }
}

pub fn handle_focus_change(
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    paused: Res<Paused>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    if paused.0 || !window.focused {
        release_cursor(&mut cursor_options);
    } else {
        lock_cursor(&mut cursor_options);
    }
}
