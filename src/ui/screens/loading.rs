use crate::AppState;
use bevy::prelude::*;

pub fn drive_asset_loading(mut next_state: ResMut<NextState<AppState>>) {
    next_state.set(AppState::MainMenu);
}
