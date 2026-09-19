use crate::data::GameData;
use crate::player::{Player, PlayerState};
use crate::world::{EditedVoxels, GameWorld};
use bevy::gizmos::prelude::Gizmos;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_voxel_world::prelude::{VoxelRaycastResult, VoxelWorld, VoxelWorldCamera, WorldVoxel};

#[derive(Resource, Default)]
pub struct BlockBreakProgress {
    target: Option<IVec3>,
    elapsed: f32,
}

#[derive(Component)]
pub struct BreakOverlay {
    stage: usize,
    textures: Vec<Handle<Image>>,
}

pub fn spawn_break_overlay(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    data: Res<GameData>,
) {
    let textures: Vec<Handle<Image>> = data
        .assets
        .break_stage_textures
        .iter()
        .map(|path| asset_server.load(path.clone()))
        .collect();
    let initial_texture = textures[0].clone();

    commands.spawn((
        BreakOverlay { stage: 0, textures },
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(initial_texture),
            alpha_mode: AlphaMode::Mask(0.99),
            ..default()
        })),
        Transform::from_scale(Vec3::splat(1.01)),
        Visibility::Hidden,
    ));
}

pub fn despawn_break_overlay(mut commands: Commands, query: Query<Entity, With<BreakOverlay>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn targeted_block(
    voxel_world: &VoxelWorld<GameWorld>,
    camera_query: &Query<(&Camera, &GlobalTransform), With<VoxelWorldCamera<GameWorld>>>,
    window_query: &Query<&Window, With<PrimaryWindow>>,
    reach: f32,
) -> Option<VoxelRaycastResult> {
    let (camera, camera_transform) = camera_query.single().ok()?;
    let window = window_query.single().ok()?;
    let screen_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
    let ray = camera
        .viewport_to_world(camera_transform, screen_center)
        .ok()?;
    let hit = voxel_world.raycast(ray, &|_| true)?;
    (hit.position.distance(ray.origin) <= reach).then_some(hit)
}

pub fn block_interaction(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut voxel_world: VoxelWorld<GameWorld>,
    camera_query: Query<(&Camera, &GlobalTransform), With<VoxelWorldCamera<GameWorld>>>,
    player_query: Query<(&Transform, &PlayerState), With<Player>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut edited_voxels: ResMut<EditedVoxels>,
    time: Res<Time>,
    mut break_progress: ResMut<BlockBreakProgress>,
    data: Res<GameData>,
) {
    let Some(hit) = targeted_block(
        &voxel_world,
        &camera_query,
        &window_query,
        data.player.reach,
    ) else {
        break_progress.reset();
        return;
    };
    let Ok((player_transform, player_state)) = player_query.single() else {
        return;
    };
    let half_extents = if player_state.crouching {
        data.player.crouching_extents()
    } else {
        data.player.standing_extents()
    };

    if mouse_buttons.pressed(MouseButton::Left) {
        let break_pos = hit.position.as_ivec3();
        let block = match voxel_world.get_voxel(break_pos) {
            WorldVoxel::Solid(block) => block,
            _ => {
                break_progress.reset();
                return;
            }
        };

        if break_progress.target != Some(break_pos) {
            break_progress.target = Some(break_pos);
            break_progress.elapsed = 0.0;
        }
        break_progress.elapsed += time.delta_secs();

        if break_progress.elapsed >= break_duration(block, &data) {
            voxel_world.set_voxel(break_pos, WorldVoxel::Air);
            edited_voxels.positions.insert(break_pos);
            break_progress.target = None;
            break_progress.elapsed = 0.0;
        }
    } else {
        break_progress.reset();
    }

    if mouse_buttons.pressed(MouseButton::Right) {
        let place_pos = (hit.position + hit.normal.unwrap_or(Vec3::Y)).as_ivec3();
        if !block_intersects_player(place_pos, player_transform.translation, half_extents) {
            voxel_world.set_voxel(place_pos, WorldVoxel::Solid(player_state.selected_block));
            edited_voxels.positions.insert(place_pos);
        }
    }
}

fn break_duration(block: u8, data: &GameData) -> f32 {
    data.block(block).break_duration
}

impl BlockBreakProgress {
    fn reset(&mut self) {
        self.target = None;
        self.elapsed = 0.0;
    }
}

fn break_stage(elapsed: f32, block: u8, data: &GameData) -> usize {
    ((elapsed / break_duration(block, data)).clamp(0.0, 0.999)
        * data.assets.break_stage_textures.len() as f32) as usize
}

pub fn block_outline(
    voxel_world: VoxelWorld<GameWorld>,
    camera_query: Query<(&Camera, &GlobalTransform), With<VoxelWorldCamera<GameWorld>>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    data: Res<GameData>,
    mut gizmos: Gizmos,
) {
    if let Some(hit) = targeted_block(
        &voxel_world,
        &camera_query,
        &window_query,
        data.player.reach,
    ) {
        draw_block_outline(&mut gizmos, hit.position.as_ivec3());
    }
}

pub fn update_break_overlay(
    break_progress: Res<BlockBreakProgress>,
    voxel_world: VoxelWorld<GameWorld>,
    images: Res<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut overlay: Single<(
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Transform,
        &mut Visibility,
        &mut BreakOverlay,
    )>,
    data: Res<GameData>,
) {
    let (material_handle, transform, visibility, state) = &mut *overlay;
    let Some(target) = break_progress.target else {
        **visibility = Visibility::Hidden;
        return;
    };

    let WorldVoxel::Solid(block) = voxel_world.get_voxel(target) else {
        **visibility = Visibility::Hidden;
        return;
    };
    let stage = break_stage(break_progress.elapsed, block, &data);
    transform.translation = target.as_vec3() + Vec3::splat(0.5);
    **visibility = Visibility::Visible;
    if state.stage != stage {
        let texture = state.textures[stage].clone();
        if images.get(&texture).is_some() {
            state.stage = stage;
            if let Some(mut material) = materials.get_mut(&material_handle.0) {
                material.base_color_texture = Some(texture.clone());
            }
        }
    }
}

fn draw_block_outline(gizmos: &mut Gizmos, block_pos: IVec3) {
    let center = block_pos.as_vec3() + Vec3::splat(0.5);
    gizmos.cube(
        Transform::from_translation(center).with_scale(Vec3::splat(1.004)),
        Color::BLACK,
    );
}

fn block_intersects_player(block_pos: IVec3, player_pos: Vec3, half_extents: Vec3) -> bool {
    let block_min = block_pos.as_vec3();
    let block_max = block_min + Vec3::ONE;
    let player_min = player_pos - half_extents;
    let player_max = player_pos + half_extents;

    block_min.x < player_max.x
        && block_max.x > player_min.x
        && block_min.y < player_max.y
        && block_max.y > player_min.y
        && block_min.z < player_max.z
        && block_max.z > player_min.z
}
