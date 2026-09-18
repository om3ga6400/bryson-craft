use crate::data::GameData;
use crate::settings::{GameSettings, WindowModeSetting};
use crate::ui::spawn_button;
use crate::world::GameWorld;
use bevy::prelude::*;
use std::sync::atomic::Ordering;

#[derive(Component)]
pub struct OptionsMenuRoot;

#[derive(Component, Clone, Copy)]
pub enum OptionsButtonAction {
    SensitivityDown,
    SensitivityUp,
    RenderDistanceDown,
    RenderDistanceUp,
    FovDown,
    FovUp,
    WindowModeCycle,
    Back,
}

#[derive(Component, Clone, Copy)]
pub enum OptionValueText {
    Sensitivity,
    RenderDistance,
    Fov,
    WindowMode,
}

fn spawn_stepper_button(row: &mut ChildSpawnerCommands, label: &str, action: OptionsButtonAction) {
    spawn_button(row, label, action, 40.0, 40.0, 22.0, UiRect::default());
}

fn spawn_option_label(parent: &mut ChildSpawnerCommands, label: &str) {
    parent.spawn((
        Text::new(label),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
        Node {
            width: Val::Px(220.0),
            height: Val::Px(24.0),
            ..default()
        },
    ));
}

fn spawn_setting_row(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    actions: [(&str, OptionsButtonAction); 2],
    value_marker: OptionValueText,
    value_text: String,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            ..default()
        })
        .with_children(|row| {
            spawn_option_label(row, label);
            for (symbol, action) in actions {
                spawn_stepper_button(row, symbol, action);
            }
            row.spawn((
                value_marker,
                Text::new(value_text),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                Node {
                    width: Val::Px(60.0),
                    height: Val::Px(24.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ));
        });
}

pub fn spawn_options_menu(commands: &mut Commands, settings: &GameSettings) {
    commands
        .spawn((
            OptionsMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
            ZIndex(100),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Options"),
                TextFont {
                    font_size: FontSize::Px(40.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            spawn_setting_row(
                parent,
                "Mouse Sensitivity",
                [
                    ("-", OptionsButtonAction::SensitivityDown),
                    ("+", OptionsButtonAction::SensitivityUp),
                ],
                OptionValueText::Sensitivity,
                format!("{:.1}x", settings.mouse_sensitivity),
            );

            spawn_setting_row(
                parent,
                "Render Distance",
                [
                    ("-", OptionsButtonAction::RenderDistanceDown),
                    ("+", OptionsButtonAction::RenderDistanceUp),
                ],
                OptionValueText::RenderDistance,
                format!("{}", settings.render_distance),
            );

            spawn_setting_row(
                parent,
                "Field of View",
                [
                    ("-", OptionsButtonAction::FovDown),
                    ("+", OptionsButtonAction::FovUp),
                ],
                OptionValueText::Fov,
                format!("{:.0}", settings.fov),
            );

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(16.0),
                    ..default()
                })
                .with_children(|row| {
                    spawn_option_label(row, "Window Mode");
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(200.0),
                            height: Val::Px(40.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        BorderColor::all(Color::srgb(0.4, 0.4, 0.4)),
                        BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                        OptionsButtonAction::WindowModeCycle,
                    ))
                    .with_child((
                        OptionValueText::WindowMode,
                        Text::new(window_mode_display(settings.window_mode)),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));
                });

            parent
                .spawn(Node {
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|parent| {
                    spawn_button(
                        parent,
                        "Back",
                        OptionsButtonAction::Back,
                        200.0,
                        50.0,
                        24.0,
                        UiRect::default(),
                    );
                });
        });
}

fn window_mode_display(mode: WindowModeSetting) -> &'static str {
    match mode {
        WindowModeSetting::Windowed => "Windowed",
        WindowModeSetting::BorderlessFullscreen => "Borderless",
        WindowModeSetting::ExclusiveFullscreen => "Fullscreen",
    }
}

fn next_window_mode(current: WindowModeSetting) -> WindowModeSetting {
    match current {
        WindowModeSetting::Windowed => WindowModeSetting::BorderlessFullscreen,
        WindowModeSetting::BorderlessFullscreen => WindowModeSetting::ExclusiveFullscreen,
        WindowModeSetting::ExclusiveFullscreen => WindowModeSetting::Windowed,
    }
}

pub fn handle_options_buttons(
    interaction_query: Query<
        (&Interaction, &OptionsButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut settings: ResMut<GameSettings>,
    game_world: Res<GameWorld>,
    data: Res<GameData>,
    mut commands: Commands,
    options_root_query: Query<Entity, With<OptionsMenuRoot>>,
    mut value_texts: Query<(&mut Text, &OptionValueText)>,
) {
    let mut any_change = false;
    for (interaction, action) in &interaction_query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        any_change = true;

        match action {
            OptionsButtonAction::SensitivityDown => {
                settings.mouse_sensitivity = (settings.mouse_sensitivity
                    - data.settings.sensitivity_step)
                    .max(data.settings.min_sensitivity);
            }
            OptionsButtonAction::SensitivityUp => {
                settings.mouse_sensitivity = (settings.mouse_sensitivity
                    + data.settings.sensitivity_step)
                    .min(data.settings.max_sensitivity);
            }
            OptionsButtonAction::RenderDistanceDown => {
                settings.render_distance = settings
                    .render_distance
                    .saturating_sub(data.settings.render_distance_step)
                    .max(data.settings.min_render_distance);
                game_world
                    .render_distance
                    .store(settings.render_distance, Ordering::Relaxed);
            }
            OptionsButtonAction::RenderDistanceUp => {
                settings.render_distance = (settings.render_distance
                    + data.settings.render_distance_step)
                    .min(data.settings.max_render_distance);
                game_world
                    .render_distance
                    .store(settings.render_distance, Ordering::Relaxed);
            }
            OptionsButtonAction::FovDown => {
                settings.fov = (settings.fov - data.settings.fov_step).max(data.settings.min_fov);
            }
            OptionsButtonAction::FovUp => {
                settings.fov = (settings.fov + data.settings.fov_step).min(data.settings.max_fov);
            }
            OptionsButtonAction::WindowModeCycle => {
                settings.window_mode = next_window_mode(settings.window_mode);
            }
            OptionsButtonAction::Back => {
                for entity in &options_root_query {
                    commands.entity(entity).despawn();
                }
            }
        }
    }

    if !any_change {
        return;
    }

    for (mut text, value_type) in &mut value_texts {
        text.0 = match value_type {
            OptionValueText::Sensitivity => format!("{:.1}x", settings.mouse_sensitivity),
            OptionValueText::RenderDistance => format!("{}", settings.render_distance),
            OptionValueText::Fov => format!("{:.0}", settings.fov),
            OptionValueText::WindowMode => window_mode_display(settings.window_mode).to_string(),
        };
    }
}
