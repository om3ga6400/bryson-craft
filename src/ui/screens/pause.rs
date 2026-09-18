use crate::AppState;
use crate::settings::GameSettings;
use crate::ui::cursor::{lock_cursor, release_cursor};
use crate::ui::screens::options::{OptionsMenuRoot, spawn_options_menu};
use crate::ui::{Paused, spawn_button};
use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};

#[derive(Component)]
pub struct PauseMenuRoot;

#[derive(Component)]
pub enum PauseButtonAction {
    Resume,
    Options,
    MainMenu,
    Quit,
}

pub fn game_is_active(paused: Res<Paused>) -> bool {
    !paused.0
}

fn spawn_pause_menu(commands: &mut Commands) {
    commands
        .spawn((
            PauseMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Paused"),
                TextFont {
                    font_size: FontSize::Px(48.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(24.0)),
                    ..default()
                },
            ));
            for (label, action) in [
                ("Resume", PauseButtonAction::Resume),
                ("Options", PauseButtonAction::Options),
                ("Main Menu", PauseButtonAction::MainMenu),
                ("Quit Game", PauseButtonAction::Quit),
            ] {
                spawn_button(parent, label, action, 200.0, 50.0, 24.0, UiRect::default());
            }
        });
}

pub fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    mut paused: ResMut<Paused>,
    mut commands: Commands,
    pause_menu_query: Query<Entity, With<PauseMenuRoot>>,
    options_menu_query: Query<Entity, With<OptionsMenuRoot>>,
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if !options_menu_query.is_empty() {
        for entity in &options_menu_query {
            commands.entity(entity).despawn();
        }
        return;
    }
    paused.0 = !paused.0;
    if paused.0 {
        release_cursor(&mut cursor_options);
        spawn_pause_menu(&mut commands);
    } else {
        lock_cursor(&mut cursor_options);
        for entity in &pause_menu_query {
            commands.entity(entity).despawn();
        }
    }
}

pub fn handle_pause_buttons(
    interaction_query: Query<
        (&Interaction, &PauseButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut paused: ResMut<Paused>,
    mut commands: Commands,
    pause_menu_query: Query<Entity, With<PauseMenuRoot>>,
    options_menu_query: Query<Entity, With<OptionsMenuRoot>>,
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    settings: Res<GameSettings>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, action) in &interaction_query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            PauseButtonAction::Resume => {
                paused.0 = false;
                lock_cursor(&mut cursor_options);
                for entity in &pause_menu_query {
                    commands.entity(entity).despawn();
                }
            }
            PauseButtonAction::Options => spawn_options_menu(&mut commands, &settings),
            PauseButtonAction::MainMenu => {
                paused.0 = false;
                release_cursor(&mut cursor_options);
                for entity in pause_menu_query.iter().chain(options_menu_query.iter()) {
                    commands.entity(entity).despawn();
                }
                next_state.set(AppState::MainMenu);
            }
            PauseButtonAction::Quit => std::process::exit(0),
        }
    }
}
