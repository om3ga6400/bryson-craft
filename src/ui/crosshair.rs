use crate::assets;
use crate::data::GameData;
use bevy::prelude::*;

#[derive(Component)]
pub struct Crosshair;

pub fn spawn_crosshair(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    data: Res<GameData>,
) {
    let mut root = commands.spawn((
        Crosshair,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
    ));

    if let Some(crosshair) = assets::resolve(&data.assets.crosshair_candidates) {
        root.with_child((
            ImageNode::new(asset_server.load(crosshair)),
            Node {
                width: Val::Px(22.0),
                height: Val::Px(22.0),
                ..default()
            },
        ));
    } else {
        root.with_child((
            Text::new("."),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
        ));
    }
}

pub fn despawn_crosshair(mut commands: Commands, query: Query<Entity, With<Crosshair>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

pub fn toggle_crosshair_visibility(
    paused: Res<crate::ui::Paused>,
    mut query: Query<&mut Visibility, With<Crosshair>>,
) {
    for mut visibility in &mut query {
        *visibility = if paused.0 {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}
