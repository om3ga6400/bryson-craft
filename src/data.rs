use bevy::prelude::{Resource, Vec3};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const DATA_FILES: [&str; 7] = [
    "assets.json",
    "blocks.json",
    "camera.json",
    "player.json",
    "settings.json",
    "simulation.json",
    "terrain.json",
];

#[derive(Clone, Debug, Deserialize)]
pub struct BlockDefinition {
    pub name: String,
    #[serde(default)]
    pub grass_tint: bool,
    #[serde(default = "default_break_duration")]
    pub break_duration: f32,
    #[serde(default)]
    pub hotbar_key: Option<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AssetDefinition {
    pub atlas: String,
    pub tile_size: u32,
    pub faces_per_block: u32,
    pub grass_tint: [u8; 3],
    pub crosshair: String,
    pub break_stage_textures: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SimulationDefinition {
    pub ticks_per_second: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlayerDefinition {
    pub standing_half_extents: [f32; 3],
    pub crouching_half_extents: [f32; 3],
    pub eye_height: f32,
    pub crouch_eye_height: f32,
    pub reach: f32,
    pub walk_speed: f32,
    pub sprint_speed: f32,
    pub crouch_speed: f32,
    pub jump_speed: f32,
    pub gravity: f32,
    pub terminal_velocity: f32,
    pub collision_step: f32,
}

impl PlayerDefinition {
    pub fn standing_extents(&self) -> Vec3 {
        Vec3::from_array(self.standing_half_extents)
    }

    pub fn crouching_extents(&self) -> Vec3 {
        Vec3::from_array(self.crouching_half_extents)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CameraDefinition {
    pub look_sensitivity: f32,
    pub max_pitch: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SettingsDefinition {
    pub default_mouse_sensitivity: f32,
    pub min_sensitivity: f32,
    pub max_sensitivity: f32,
    pub sensitivity_step: f32,
    pub default_render_distance: u32,
    pub min_render_distance: u32,
    pub max_render_distance: u32,
    pub render_distance_step: u32,
    pub default_fov: f32,
    pub min_fov: f32,
    pub max_fov: f32,
    pub fov_step: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TerrainDefinition {
    pub seed: u32,
    pub scale: f64,
    pub height: f64,
    #[serde(default = "default_surface_block")]
    pub surface: usize,
    #[serde(default = "default_dirt_block")]
    pub subsurface: usize,
    #[serde(default = "default_dirt_layers")]
    pub subsurface_layers: i32,
    #[serde(default = "default_stone_block")]
    pub underground: usize,
}

#[derive(Clone, Debug, Resource)]
pub struct GameData {
    pub assets: AssetDefinition,
    pub simulation: SimulationDefinition,
    pub player: PlayerDefinition,
    pub camera: CameraDefinition,
    pub settings: SettingsDefinition,
    pub blocks: Vec<BlockDefinition>,
    pub terrain: TerrainDefinition,
}

impl GameData {
    pub fn load() -> Self {
        let directory = data_dir();
        let assets: AssetDefinition = load_file(&directory, "assets.json");
        let simulation: SimulationDefinition = load_file(&directory, "simulation.json");
        let player: PlayerDefinition = load_file(&directory, "player.json");
        let camera: CameraDefinition = load_file(&directory, "camera.json");
        let settings: SettingsDefinition = load_file(&directory, "settings.json");
        let blocks: Vec<BlockDefinition> = load_file(&directory, "blocks.json");
        let terrain: TerrainDefinition = load_file(&directory, "terrain.json");
        let data = Self {
            assets,
            simulation,
            player,
            camera,
            settings,
            blocks,
            terrain,
        };
        data.validate(&directory);
        data
    }

    pub fn block(&self, id: u8) -> &BlockDefinition {
        &self.blocks[id as usize]
    }
}

pub fn ensure_data_files() {
    let directory = data_dir();
    fs::create_dir_all(&directory).expect("Could not create data directory");

    for filename in DATA_FILES {
        let path = directory.join(filename);
        if path.is_file() {
            tracing::info!("data/{filename}: already present");
            continue;
        }

        let url = format!(
            "https://raw.githubusercontent.com/om3ga6400/bryson-craft/main/data/{filename}"
        );
        let status = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!(
                    "Invoke-WebRequest -UseBasicParsing -Uri '{}' -OutFile '{}'",
                    url,
                    path.display()
                ),
            ])
            .status()
            .unwrap_or_else(|error| panic!("Could not download {filename}: {error}"));
        assert!(status.success(), "Could not download {filename}");
        tracing::info!("data/{filename}: downloaded");
    }
}

pub fn data_folder_exists() -> bool {
    data_dir().is_dir()
}

pub fn data_files_are_valid() -> bool {
    let directory = data_dir();
    let mut valid = true;
    for filename in DATA_FILES {
        let file_valid = fs::read_to_string(directory.join(filename))
            .ok()
            .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok())
            .is_some();
        if file_valid {
            tracing::info!("validation success data/{filename}");
        } else {
            tracing::error!("validation failure data/{filename}");
        }
        valid &= file_valid;
    }
    valid
}

impl GameData {
    fn validate(&self, path: &Path) {
        assert!(
            !self.blocks.is_empty(),
            "{} defines no blocks",
            path.display()
        );
        assert!(
            !self.assets.atlas.is_empty(),
            "{} defines no atlas path",
            path.display()
        );
        assert!(
            self.assets.tile_size > 0,
            "{} defines an invalid tile size",
            path.display()
        );
        assert!(
            self.assets.faces_per_block > 0,
            "{} defines no block faces",
            path.display()
        );
        assert!(
            !self.assets.break_stage_textures.is_empty(),
            "{} defines no break stage textures",
            path.display()
        );
        assert!(self.simulation.ticks_per_second > 0.0);
        assert!(self.player.collision_step > 0.0);
        assert!(self.player.reach > 0.0);
        assert!(self.camera.look_sensitivity > 0.0);
        assert!(self.settings.sensitivity_step > 0.0);
        assert!(self.settings.render_distance_step > 0);
        assert!(self.settings.fov_step > 0.0);
        assert!(
            self.blocks.len() <= u8::MAX as usize + 1,
            "{} defines too many blocks for u8 voxel IDs",
            path.display()
        );
        let mut hotbar_keys = HashSet::new();
        for (index, block) in self.blocks.iter().enumerate() {
            assert!(!block.name.is_empty(), "block {index} has no name");
            assert!(
                !block.name.contains('/')
                    && !block.name.contains('\\')
                    && block.name != "."
                    && block.name != "..",
                "block {index} has an invalid texture name"
            );
            assert!(
                block.break_duration > 0.0,
                "block {index} has invalid break duration"
            );
            if let Some(key) = block.hotbar_key {
                assert!(
                    (1..=9).contains(&key),
                    "block {index} has invalid hotbar key"
                );
                assert!(
                    hotbar_keys.insert(key),
                    "{} assigns hotbar key {key} more than once",
                    path.display()
                );
            }
        }
        for (name, index) in [
            ("surface", self.terrain.surface),
            ("subsurface", self.terrain.subsurface),
            ("underground", self.terrain.underground),
        ] {
            assert!(
                index < self.blocks.len(),
                "terrain {name} block is out of range"
            );
        }
        assert!(
            self.terrain.scale.is_finite() && self.terrain.scale > 0.0,
            "{} defines an invalid terrain scale",
            path.display()
        );
        assert!(
            self.terrain.height.is_finite() && self.terrain.height > 0.0,
            "{} defines an invalid terrain height",
            path.display()
        );
        assert!(
            self.terrain.subsurface_layers >= 0,
            "{} defines a negative subsurface layer count",
            path.display()
        );
    }
}

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
}

fn load_file<T: DeserializeOwned>(directory: &Path, filename: &str) -> T {
    let path = directory.join(filename);
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("Could not read {}: {error}", path.display()));
    serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("Could not parse {}: {error}", path.display()))
}

fn default_break_duration() -> f32 {
    0.75
}

fn default_surface_block() -> usize {
    0
}

fn default_dirt_block() -> usize {
    1
}

fn default_dirt_layers() -> i32 {
    3
}

fn default_stone_block() -> usize {
    2
}
