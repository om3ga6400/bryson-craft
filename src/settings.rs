use crate::data::GameData;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, VideoModeSelection, WindowMode};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WindowModeSetting {
    Windowed,
    #[default]
    BorderlessFullscreen,
    ExclusiveFullscreen,
}

#[derive(Resource)]
pub struct GameSettings {
    pub mouse_sensitivity: f32,
    pub render_distance: u32,
    pub fov: f32,
    pub window_mode: WindowModeSetting,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self::from_data(&GameData::load())
    }
}

impl GameSettings {
    pub fn from_data(data: &GameData) -> Self {
        Self {
            mouse_sensitivity: data.settings.default_mouse_sensitivity,
            render_distance: data.settings.default_render_distance,
            fov: data.settings.default_fov,
            window_mode: WindowModeSetting::default(),
        }
    }
}

pub fn apply_window_mode(
    settings: Res<GameSettings>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    if !settings.is_changed() {
        return;
    }
    window.mode = match settings.window_mode {
        WindowModeSetting::Windowed => WindowMode::Windowed,
        WindowModeSetting::BorderlessFullscreen => {
            WindowMode::BorderlessFullscreen(MonitorSelection::Primary)
        }
        WindowModeSetting::ExclusiveFullscreen => {
            WindowMode::Fullscreen(MonitorSelection::Primary, VideoModeSelection::Current)
        }
    };
}
