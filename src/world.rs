use crate::data::GameData;
use crate::player::{Player, PlayerState};
use crate::settings::GameSettings;
use bevy::prelude::*;
use bevy_voxel_world::prelude::*;
use noise::{NoiseFn, Perlin};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

pub fn terrain_height(perlin: &Perlin, x: i32, z: i32, data: &GameData) -> i32 {
    (perlin.get([x as f64 * data.terrain.scale, z as f64 * data.terrain.scale])
        * data.terrain.height) as i32
}

pub fn terrain_block(perlin: &Perlin, pos: IVec3, data: &GameData) -> Option<u8> {
    let height = terrain_height(perlin, pos.x, pos.z, data);
    if pos.y > height {
        return None;
    }
    Some(match height - pos.y {
        0 => data.terrain.surface as u8,
        depth if depth <= data.terrain.subsurface_layers => data.terrain.subsurface as u8,
        _ => data.terrain.underground as u8,
    })
}

pub fn terrain_voxel(perlin: &Perlin, pos: IVec3, data: &GameData) -> WorldVoxel<u8> {
    terrain_block(perlin, pos, data).map_or(WorldVoxel::Unset, WorldVoxel::Solid)
}

#[derive(Resource, Clone)]
pub struct GameWorld {
    pub data: Arc<GameData>,
    pub seed: Arc<AtomicU32>,
    pub render_distance: Arc<AtomicU32>,
}

impl GameWorld {
    pub fn new(data: GameData) -> Self {
        let render_distance = data.settings.default_render_distance;
        Self {
            seed: Arc::new(AtomicU32::new(data.terrain.seed)),
            data: Arc::new(data),
            render_distance: Arc::new(AtomicU32::new(render_distance)),
        }
    }
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new(GameData::load())
    }
}

impl VoxelWorldConfig for GameWorld {
    type MaterialIndex = u8;
    type ChunkUserBundle = ();

    fn spawning_distance(&self) -> u32 {
        self.render_distance.load(Ordering::Relaxed)
    }

    fn voxel_texture(&self) -> Option<(String, u32)> {
        Some((
            self.data.assets.atlas.clone(),
            self.data.blocks.len() as u32 * self.data.assets.faces_per_block,
        ))
    }

    fn texture_index_mapper(&self) -> Arc<dyn Fn(u8) -> [u32; 3] + Send + Sync> {
        let block_count = self.data.blocks.len() as u32;
        let faces_per_block = self.data.assets.faces_per_block;
        Arc::new(move |block| {
            let base = (block as u32).min(block_count - 1) * faces_per_block;
            [base, base + 1, base + 2]
        })
    }

    fn voxel_lookup_delegate(&self) -> VoxelLookupDelegate<Self::MaterialIndex> {
        let seed_handle = self.seed.clone();
        let data = self.data.clone();
        Box::new(move |_, _, _| {
            let perlin = Perlin::new(seed_handle.load(Ordering::Relaxed));
            let lookup_data = data.clone();
            Box::new(move |pos: IVec3, _| terrain_voxel(&perlin, pos, &lookup_data))
        })
    }
}

pub fn spawn_game_world(
    mut commands: Commands,
    mut clear_color: ResMut<ClearColor>,
    game_world: Res<GameWorld>,
    settings: Res<GameSettings>,
) {
    clear_color.0 = Color::srgb(0.53, 0.81, 0.92);

    let noise = Perlin::new(game_world.seed.load(Ordering::Relaxed));
    let spawn_pos = Vec3::new(
        0.0,
        terrain_height(&noise, 0, 0, &game_world.data) as f32 + 3.0,
        0.0,
    );

    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            -std::f32::consts::FRAC_PI_4,
            0.0,
        )),
    ));

    commands.spawn((
        Player,
        PlayerState {
            prev_position: spawn_pos,
            ..default()
        },
        Transform::from_translation(spawn_pos),
    ));

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: settings.fov.to_radians(),
            ..default()
        }),
        Transform::from_translation(spawn_pos + Vec3::Y * game_world.data.player.eye_height),
        AmbientLight {
            color: Color::WHITE,
            brightness: 300.0,
            ..default()
        },
        VoxelWorldCamera::<GameWorld>::default(),
    ));
}

#[derive(Resource, Default)]
pub struct EditedVoxels {
    pub positions: HashSet<IVec3>,
}

pub fn despawn_game_world(
    mut commands: Commands,
    mut clear_color: ResMut<ClearColor>,
    game_entities: Query<Entity, Or<(With<Player>, With<Camera3d>, With<DirectionalLight>)>>,
    chunks: Query<Entity, With<Chunk<GameWorld>>>,
) {
    clear_color.0 = Color::srgb(0.12, 0.12, 0.12);

    for entity in &game_entities {
        commands.entity(entity).despawn();
    }

    for entity in &chunks {
        commands.entity(entity).insert(NeedsDespawn);
    }
}

pub fn reset_edited_voxels(
    mut voxel_world: VoxelWorld<GameWorld>,
    mut edited_voxels: ResMut<EditedVoxels>,
    game_world: Res<GameWorld>,
) {
    if edited_voxels.positions.is_empty() {
        return;
    }
    let perlin = Perlin::new(game_world.seed.load(Ordering::Relaxed));
    for position in edited_voxels.positions.drain() {
        voxel_world.set_voxel(position, terrain_voxel(&perlin, position, &game_world.data));
    }
}
