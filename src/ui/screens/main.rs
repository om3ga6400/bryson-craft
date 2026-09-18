use crate::AppState;
use crate::settings::GameSettings;
use crate::ui::screens::options::{OptionsMenuRoot, spawn_options_menu};
use crate::ui::spawn_button;
use bevy::prelude::*;

#[derive(Component)]
pub struct MainMenuRoot;

#[derive(Component)]
pub enum MenuButtonAction {
    Singleplayer,
    Options,
    Quit,
}

pub fn setup_main_menu(mut commands: Commands) {
    commands.spawn((MainMenuRoot, Camera2d));

    commands
        .spawn((
            MainMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.12)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("BrysonCraft"),
                TextFont {
                    font_size: FontSize::Px(80.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|buttons| {
                    for (label, action) in [
                        ("Singleplayer", MenuButtonAction::Singleplayer),
                        ("Options", MenuButtonAction::Options),
                        ("Quit Game", MenuButtonAction::Quit),
                    ] {
                        spawn_button(buttons, label, action, 200.0, 50.0, 24.0, UiRect::default());
                    }
                });

            parent.spawn((
                Text::new("v0.1.0"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(10.0),
                    right: Val::Px(10.0),
                    ..default()
                },
            ));
        });
}

pub fn handle_menu_buttons(
    interaction_query: Query<
        (&Interaction, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    settings: Res<GameSettings>,
) {
    for (interaction, action) in &interaction_query {
        if *interaction == Interaction::Pressed {
            match action {
                MenuButtonAction::Singleplayer => next_state.set(AppState::Loading),
                MenuButtonAction::Options => spawn_options_menu(&mut commands, &settings),
                MenuButtonAction::Quit => std::process::exit(0),
            }
        }
    }
}

pub fn despawn_main_menu(
    mut commands: Commands,
    main_menu_query: Query<Entity, With<MainMenuRoot>>,
    options_menu_query: Query<Entity, With<OptionsMenuRoot>>,
) {
    for entity in main_menu_query.iter().chain(options_menu_query.iter()) {
        commands.entity(entity).despawn();
    }
}
