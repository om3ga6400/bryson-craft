#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod assets;
mod data;
mod player;
mod settings;
mod ui;
mod world;

use bevy::gizmos::prelude::{DefaultGizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};
use bevy_voxel_world::prelude::VoxelWorldPlugin;

use player::input::{mouse_look, read_input, sync_camera};
use player::interaction::{
    BlockBreakProgress, block_interaction, block_outline, despawn_break_overlay,
    spawn_break_overlay, update_break_overlay,
};
use player::physics::step_player_physics;
use settings::{GameSettings, apply_window_mode};
use ui::{
    AssetLoadProgress, Paused, despawn_crosshair, despawn_main_menu, drive_asset_loading,
    game_is_active, grab_cursor, handle_focus_change, handle_menu_buttons, handle_options_buttons,
    handle_pause_buttons, manage_cursor, setup_main_menu, spawn_crosshair,
    toggle_crosshair_visibility, toggle_pause,
};
use world::{EditedVoxels, GameWorld, despawn_game_world, reset_edited_voxels, spawn_game_world};

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    AssetLoading,
    MainMenu,
    Loading,
    InGame,
}

fn configure_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = 2.0;
}

fn show_window(mut window: Single<&mut Window, With<PrimaryWindow>>) {
    window.visible = true;
}

pub fn enter_loading(
    game_world: Res<GameWorld>,
    settings: Res<GameSettings>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    game_world
        .seed
        .store(rand::random(), std::sync::atomic::Ordering::Relaxed);
    game_world.render_distance.store(
        settings.render_distance,
        std::sync::atomic::Ordering::Relaxed,
    );
    next_state.set(AppState::InGame);
}

fn main() {
    let game_data = data::GameData::load();

    assets::ensure_atlas_exists();

    App::new()
        .add_plugins((
            DefaultPlugins
                .set(bevy::window::WindowPlugin {
                    primary_window: Some(bevy::window::Window {
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                        visible: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            VoxelWorldPlugin::with_config(GameWorld::new(game_data.clone())),
        ))
        .insert_resource(Time::<Fixed>::from_hz(
            game_data.simulation.ticks_per_second,
        ))
        .init_state::<AppState>()
        .insert_resource(GameSettings::from_data(&game_data))
        .insert_resource(game_data)
        .init_resource::<Paused>()
        .init_resource::<BlockBreakProgress>()
        .init_resource::<EditedVoxels>()
        .init_resource::<AssetLoadProgress>()
        .insert_resource(ClearColor(Color::srgb(0.12, 0.12, 0.12)))
        .add_systems(Startup, configure_gizmos)
        .add_systems(
            Update,
            drive_asset_loading.run_if(in_state(AppState::AssetLoading)),
        )
        .add_systems(
            OnEnter(AppState::MainMenu),
            (show_window, setup_main_menu).chain(),
        )
        .add_systems(OnExit(AppState::MainMenu), despawn_main_menu)
        .add_systems(
            Update,
            handle_menu_buttons.run_if(in_state(AppState::MainMenu)),
        )
        .add_systems(
            OnEnter(AppState::Loading),
            (enter_loading, reset_edited_voxels, spawn_game_world).chain(),
        )
        .add_systems(
            OnEnter(AppState::InGame),
            (grab_cursor, spawn_crosshair, spawn_break_overlay).chain(),
        )
        .add_systems(
            OnExit(AppState::InGame),
            (despawn_crosshair, despawn_break_overlay, despawn_game_world),
        )
        .add_systems(
            Update,
            (
                (
                    read_input.run_if(game_is_active),
                    mouse_look.run_if(game_is_active),
                    sync_camera,
                    toggle_crosshair_visibility,
                    update_break_overlay,
                    block_outline.run_if(game_is_active),
                )
                    .chain(),
                (
                    manage_cursor,
                    handle_focus_change,
                    toggle_pause,
                    handle_pause_buttons,
                )
                    .chain(),
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            FixedUpdate,
            (step_player_physics, block_interaction)
                .chain()
                .run_if(game_is_active)
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(Update, (handle_options_buttons, apply_window_mode).chain())
        .run();
}
