use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::data::{BlockDefinition, GameData};
use serde_json::Value;

pub fn assets_dir() -> PathBuf {
    project_root().join("assets")
}

fn minecraft_dir() -> PathBuf {
    project_root().join("minecraft")
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn assets_folder_exists() -> bool {
    assets_dir().is_dir()
}

pub fn ensure_assets(data: &GameData) {
    let directory = assets_dir();
    let assets_exist = directory.is_dir();

    if assets_exist {
        tracing::info!("validating assets...");
        let assets_valid = assets_are_complete(data);
        tracing::info!(
            "validate assets {}",
            if assets_valid { "success" } else { "failure" }
        );
        if assets_valid {
            return;
        }
    }

    tracing::info!("requesting jar...");
    let jar = pick_jar();
    let built = build_assets(&jar, &directory)
        .map(|report| report.faces > 0)
        .unwrap_or(false);
    assert!(
        built,
        "Could not build assets from the selected Minecraft jar"
    );

    tracing::info!("validating assets...");
    let assets_valid = assets_are_complete(data);
    tracing::info!(
        "validate assets {}",
        if assets_valid { "success" } else { "failure" }
    );
    assert!(
        assets_valid,
        "Selected Minecraft jar did not provide all required assets"
    );
}

fn assets_are_complete(data: &GameData) -> bool {
    let directory = assets_dir();
    let mut complete = validate_asset_file(&directory, &data.assets.atlas);

    for path in &data.assets.break_stage_textures {
        complete &= validate_asset_file(&directory, path);
    }

    complete &= validate_asset_file(&directory, &data.assets.crosshair);

    for block in &data.blocks {
        let model = format!("models/block/{}.json", block.name);
        let block_valid = validate_asset_file(&directory, &model);
        complete &= block_valid;
    }

    complete
}

fn validate_asset_file(assets_dir: &Path, relative: &str) -> bool {
    let exists = assets_dir.join(relative).is_file();
    if exists {
        tracing::info!("validation success assets/{relative}");
    } else {
        tracing::error!("validation failure assets/{relative}");
    }
    exists
}

pub fn pick_jar() -> PathBuf {
    let picked = rfd::FileDialog::new()
        .set_title("Select your Minecraft Java Edition .jar")
        .add_filter("Minecraft jar", &["jar"])
        .pick_file()
        .expect("Minecraft jar selection was cancelled");

    let file = fs::File::open(&picked).expect("Could not open selected Minecraft jar");
    zip::ZipArchive::new(file).expect("Selected file is not a valid Minecraft jar");

    picked
}

struct Report {
    faces: usize,
}

fn build_assets(jar_path: &Path, assets_dir: &Path) -> Result<Report, Box<dyn std::error::Error>> {
    let data = GameData::load();
    let mut jar = zip::ZipArchive::new(fs::File::open(jar_path)?)?;
    let staging_dir = minecraft_dir();
    tracing::info!("extracting Minecraft assets...");
    let extracted_files = extract_assets(&mut jar, &staging_dir)?;
    tracing::info!("extracted {extracted_files} files");
    copy_model_files(&staging_dir, assets_dir, &data)?;
    let block_textures = data
        .blocks
        .iter()
        .map(|block| block_textures(assets_dir, block))
        .collect::<Result<Vec<_>, _>>()?;
    copy_required_assets(&staging_dir, assets_dir, &data, &block_textures)?;

    let texture_count = data.blocks.len() as u32 * data.assets.faces_per_block;
    tracing::info!(
        "building atlas ({} blocks, {}x{} pixels)...",
        data.blocks.len(),
        data.assets.tile_size,
        data.assets.tile_size * texture_count
    );
    let mut atlas =
        image::RgbaImage::new(data.assets.tile_size, data.assets.tile_size * texture_count);
    let mut faces = 0;

    for (index, (block, textures)) in data.blocks.iter().zip(&block_textures).enumerate() {
        tracing::info!(
            "building atlas: {}/{} ({})",
            index + 1,
            data.blocks.len(),
            block.name
        );
        let base = index as u32 * data.assets.faces_per_block;
        let (tiles, extracted) = block_tiles(assets_dir, block, textures, &data);
        for (offset, tile) in tiles.into_iter().enumerate() {
            image::imageops::overlay(
                &mut atlas,
                &tile,
                0,
                ((base + offset as u32) * data.assets.tile_size) as i64,
            );
        }
        faces += extracted;
    }

    tracing::info!("saving atlas: {}", data.assets.atlas);
    atlas.save(assets_dir.join(&data.assets.atlas))?;
    tracing::info!("atlas built with {faces} faces");
    Ok(Report { faces })
}

fn extract_assets(
    jar: &mut zip::ZipArchive<fs::File>,
    staging_dir: &Path,
) -> Result<usize, Box<dyn std::error::Error>> {
    let _ = fs::remove_dir_all(staging_dir);
    fs::create_dir_all(staging_dir)?;

    let total = jar.len();
    let mut count = 0;
    for index in 0..total {
        let mut file = jar.by_index(index)?;
        let Some(path) = file.enclosed_name() else {
            continue;
        };
        let minecraft_prefix = Path::new("assets").join("minecraft");
        if file.is_dir() || !path.starts_with(&minecraft_prefix) {
            continue;
        }
        let Ok(relative) = path.strip_prefix(&minecraft_prefix) else {
            continue;
        };
        let target = staging_dir.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        io::copy(&mut file, &mut fs::File::create(target)?)?;
        count += 1;
    }
    Ok(count)
}

fn copy_required_assets(
    staging_dir: &Path,
    assets_dir: &Path,
    data: &GameData,
    block_textures: &[[String; 3]],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut required = BTreeSet::new();
    for textures in block_textures {
        for texture in textures {
            required.insert(texture.clone());
        }
    }
    for block in &data.blocks {
        if block.grass_tint {
            required.insert(format!("textures/block/{}_side_overlay.png", block.name));
        }
    }
    for path in data
        .assets
        .break_stage_textures
        .iter()
        .chain(std::iter::once(&data.assets.crosshair))
    {
        required.insert(path.clone());
    }

    let required_count = required.len();
    let mut copied_count = 0;
    for relative in required {
        let target = assets_dir.join(&relative);
        let source = staging_dir.join(&relative);
        if source.is_file() {
            if target.is_file() {
                fs::remove_file(&target)?;
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(source, target)?;
            copied_count += 1;
            tracing::info!("copy assets/{relative}");
        } else {
            tracing::error!("copy assets/{relative}: missing in Minecraft jar");
        }
    }
    tracing::info!("copied {copied_count}/{required_count} required assets");
    fs::remove_dir_all(staging_dir)?;
    Ok(())
}

fn copy_model_files(
    staging_dir: &Path,
    assets_dir: &Path,
    data: &GameData,
) -> Result<(), Box<dyn std::error::Error>> {
    let target_dir = assets_dir.join("models").join("block");
    let source_dir = staging_dir.join("models/block");
    fs::create_dir_all(&target_dir)?;

    for block in &data.blocks {
        let filename = format!("{}.json", block.name);
        let source = source_dir.join(&filename);
        let target = target_dir.join(&filename);
        fs::copy(&source, &target)?;
        tracing::info!("copy assets/models/block/{filename}");
    }
    Ok(())
}

fn block_tiles(
    assets_dir: &Path,
    block: &BlockDefinition,
    textures: &[String; 3],
    data: &GameData,
) -> ([image::RgbaImage; 3], usize) {
    let mut extracted = 0;
    let mut tiles = textures.each_ref().map(|texture| {
        extracted += 1;
        read_texture(assets_dir, texture)
    });

    if block.grass_tint {
        tint_grass(&mut tiles[0], &data.assets.grass_tint);
        let overlay_name = format!("textures/block/{}_side_overlay.png", block.name);
        let mut overlay = read_texture(assets_dir, &overlay_name);
        tint_grass(&mut overlay, &data.assets.grass_tint);
        image::imageops::overlay(&mut tiles[1], &overlay, 0, 0);
    }

    (tiles, extracted)
}

fn read_texture(assets_dir: &Path, texture: &str) -> image::RgbaImage {
    let path = assets_dir.join(texture);
    if path.is_file() {
        let bytes = fs::read(path).expect("Could not read texture");
        return image::load_from_memory(&bytes)
            .expect("Could not decode texture")
            .to_rgba8();
    }
    panic!("Texture does not exist: {}", display_path(&path));
}

fn block_textures(
    assets_dir: &Path,
    block: &BlockDefinition,
) -> Result<[String; 3], Box<dyn std::error::Error>> {
    let path = assets_dir
        .join("models/block")
        .join(format!("{}.json", block.name));
    tracing::info!("read model {}", display_path(&path));
    let model: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
    let textures = model
        .get("textures")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("Model has no textures: {}", display_path(&path)))?;
    let top = model_texture(textures, &["up", "top", "end", "all"])?;
    let side = model_texture(textures, &["side", "north", "all"])?;
    let bottom = model_texture(textures, &["down", "bottom", "end", "all"])?;
    Ok([top, side, bottom])
}

fn model_texture(
    textures: &serde_json::Map<String, Value>,
    names: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    let value = names
        .iter()
        .find_map(|name| textures.get(*name))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Model is missing texture face: {}", names[0]))?;
    let texture = value.strip_prefix("minecraft:").unwrap_or(value);
    let texture = texture.strip_suffix(".png").unwrap_or(texture);
    let texture = texture.strip_prefix("textures/").unwrap_or(texture);
    Ok(format!("textures/{texture}.png"))
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn tint_grass(texture: &mut image::RgbaImage, tint: &[u8; 3]) {
    for pixel in texture.pixels_mut() {
        for (channel, value) in tint.iter().enumerate() {
            pixel.0[channel] = (pixel.0[channel] as u16 * *value as u16 / 255) as u8;
        }
    }
}
