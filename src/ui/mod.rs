pub mod crosshair;
pub mod cursor;
pub mod screens;

use bevy::prelude::*;

pub fn spawn_button<A: Component>(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: A,
    width: f32,
    height: f32,
    font_size: f32,
    margin: UiRect,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(width),
                height: Val::Px(height),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                margin,
                ..default()
            },
            BorderColor::all(Color::srgb(0.4, 0.4, 0.4)),
            BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            action,
        ))
        .with_child((
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
        ));
}

#[derive(Resource, Default)]
pub struct Paused(pub bool);

pub use crosshair::*;
pub use cursor::*;
pub use screens::loading::*;
pub use screens::main::*;
pub use screens::options::*;
pub use screens::pause::*;
