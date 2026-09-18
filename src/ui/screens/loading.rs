use crate::AppState;
use crate::assets;
use crate::data::GameData;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct AssetLoadProgress {
    receiver: Option<std::sync::Mutex<std::sync::mpsc::Receiver<assets::AssetProgress>>>,
}

pub fn drive_asset_loading(
    mut load: ResMut<AssetLoadProgress>,
    mut next_state: ResMut<NextState<AppState>>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    data: Res<GameData>,
) {
    let Some(receiver) = &load.receiver else {
        let jar = (!assets::existing_assets_are_complete(&data))
            .then(assets::pick_jar)
            .flatten();

        let (sender, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || assets::prepare_assets_with_progress(jar, &sender));
        load.receiver = Some(std::sync::Mutex::new(rx));
        return;
    };

    let mut finished = None;
    let rx = receiver.lock().unwrap();
    for msg in rx.try_iter() {
        match msg {
            assets::AssetProgress::Extracting { current, total } => {
                println!("Extracting assets ({current}/{total})");
            }
            assets::AssetProgress::BuildingAtlas { current, total } => {
                println!("Building texture atlas ({current}/{total})");
            }
            assets::AssetProgress::Finished { rebuilt } => {
                finished = Some(rebuilt);
            }
        }
    }

    if let Some(rebuilt) = finished {
        if rebuilt {
            reload_atlas_texture(&asset_server, &mut images, &data);
        }
        next_state.set(AppState::MainMenu);
    }
}

fn reload_atlas_texture(asset_server: &AssetServer, images: &mut Assets<Image>, data: &GameData) {
    let Ok(bytes) = std::fs::read(assets::assets_dir().join(&data.assets.atlas)) else {
        return;
    };
    let Ok(decoded) = image::load_from_memory(&bytes) else {
        return;
    };
    let handle: Handle<Image> = asset_server.load(data.assets.atlas.clone());
    if let Some(mut texture) = images.get_mut(&handle) {
        texture.data = Some(decoded.to_rgba8().into_raw());
    }
}
