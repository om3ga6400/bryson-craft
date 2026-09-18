use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use crate::data::{BlockDefinition, GameData};

pub enum AssetProgress {
    Extracting { current: usize, total: usize },
    BuildingAtlas { current: usize, total: usize },
    Finished { rebuilt: bool },
}

pub fn assets_dir() -> PathBuf {
    project_root().join("assets")
}

pub fn minecraft_dir() -> PathBuf {
    project_root().join("minecraft")
}

fn project_root() -> PathBuf {
    std::env::var_os("BEVY_ASSET_ROOT")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from))
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf))
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn resolve(candidates: &[String]) -> Option<String> {
    let dir = assets_dir();
    candidates
        .iter()
        .find(|path| dir.join(path).is_file())
        .map(|path| path.to_string())
}

pub fn existing_assets_are_complete(data: &GameData) -> bool {
    let dir = assets_dir();
    dir.join(&data.assets.atlas).is_file()
        && data
            .assets
            .break_stage_textures
            .iter()
            .all(|path| dir.join(path).is_file())
        && data
            .assets
            .crosshair_candidates
            .iter()
            .any(|path| dir.join(path).is_file())
        && data.blocks.iter().all(|block| {
            [
                top_candidates(block),
                side_candidates(block),
                bottom_candidates(block),
            ]
            .into_iter()
            .all(|candidates| {
                candidates.iter().any(|name| {
                    dir.join(&data.assets.texture_dir)
                        .join(format!("{name}.png"))
                        .is_file()
                })
            })
        })
}

pub fn ensure_atlas_exists() {
    let data = GameData::load();
    let atlas = assets_dir().join(&data.assets.atlas);
    if !atlas.is_file() {
        write_fallback_atlas(&atlas, &data);
    }
}

pub fn pick_jar() -> Option<PathBuf> {
    let picked = rfd::FileDialog::new()
        .set_title("Select your Minecraft Java Edition .jar")
        .add_filter("Minecraft jar", &["jar"])
        .pick_file()?;

    fs::File::open(&picked)
        .ok()
        .and_then(|file| zip::ZipArchive::new(file).ok())?;

    Some(picked)
}

pub fn prepare_assets_with_progress(jar: Option<PathBuf>, progress: &Sender<AssetProgress>) {
    let assets_dir = assets_dir();
    let data = GameData::load();

    let Some(jar) = jar else {
        eprintln!("Minecraft jar not provided - using existing assets.");
        let _ = progress.send(AssetProgress::Finished { rebuilt: false });
        return;
    };

    match build_assets(&jar, &assets_dir, progress) {
        Ok(report) if report.faces == 0 => {
            eprintln!("No matching textures in that jar - using fallback colors.");
            write_fallback_atlas(&assets_dir.join(&data.assets.atlas), &data);
        }
        Ok(report) => {
            let Report { files, faces } = report;
            let total_faces = data.blocks.len() * data.assets.faces_per_block as usize;
            if faces == total_faces {
                println!("Using Minecraft assets ({files} files).");
            } else {
                println!(
                    "Using Minecraft assets ({faces}/{total_faces} atlas faces, {files} files)."
                );
            }
        }
        Err(err) => {
            let _ = fs::remove_dir_all(minecraft_dir());
            eprintln!("Could not read the Minecraft jar ({err}) - using fallback colors.");
            write_fallback_atlas(&assets_dir.join(&data.assets.atlas), &data);
        }
    }
    let _ = progress.send(AssetProgress::Finished { rebuilt: true });
}

struct Report {
    files: usize,
    faces: usize,
}

fn build_assets(
    jar_path: &Path,
    assets_dir: &Path,
    progress: &Sender<AssetProgress>,
) -> Result<Report, Box<dyn std::error::Error>> {
    let data = GameData::load();
    let mut jar = zip::ZipArchive::new(fs::File::open(jar_path)?)?;
    let staging_dir = minecraft_dir();
    let files = extract_assets(&mut jar, &staging_dir, progress)?;
    copy_required_assets(&staging_dir, assets_dir, &data)?;

    let texture_count = data.blocks.len() as u32 * data.assets.faces_per_block;
    let mut atlas =
        image::RgbaImage::new(data.assets.tile_size, data.assets.tile_size * texture_count);
    let mut faces = 0;

    for (index, block) in data.blocks.iter().enumerate() {
        let _ = progress.send(AssetProgress::BuildingAtlas {
            current: index + 1,
            total: data.blocks.len(),
        });
        let base = index as u32 * data.assets.faces_per_block;
        let (tiles, extracted) = block_tiles(assets_dir, block, &data);
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

    atlas.save(assets_dir.join(&data.assets.atlas))?;
    Ok(Report { files, faces })
}

fn extract_assets(
    jar: &mut zip::ZipArchive<fs::File>,
    staging_dir: &Path,
    progress: &Sender<AssetProgress>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let _ = fs::remove_dir_all(staging_dir);
    fs::create_dir_all(staging_dir)?;

    let total = jar.len();
    let mut count = 0;
    for index in 0..total {
        if index % 256 == 0 || index + 1 == total {
            let _ = progress.send(AssetProgress::Extracting {
                current: index + 1,
                total,
            });
        }
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
) -> Result<(), Box<dyn std::error::Error>> {
    let mut required = BTreeSet::new();
    for block in &data.blocks {
        required.insert(format!("textures/block/{}.png", block.name));
        required.insert(format!("textures/block/{}_top.png", block.name));
        required.insert(format!("textures/block/{}_side.png", block.name));
        required.insert(format!("textures/block/{}_side_overlay.png", block.name));
        required.insert(format!("textures/block/{}_bottom.png", block.name));
        if let Some(bottom) = &block.bottom {
            required.insert(format!("textures/block/{bottom}.png"));
        }
    }
    for path in data
        .assets
        .break_stage_textures
        .iter()
        .chain(data.assets.crosshair_candidates.iter())
    {
        required.insert(path.clone());
    }

    for relative in required {
        let target = assets_dir.join(&relative);
        if target.is_file() {
            fs::remove_file(&target)?;
        }
        let source = staging_dir.join(&relative);
        if !source.is_file() {
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(source, target)?;
    }
    fs::remove_dir_all(staging_dir)?;
    Ok(())
}

fn block_tiles(
    assets_dir: &Path,
    block: &BlockDefinition,
    data: &GameData,
) -> ([image::RgbaImage; 3], usize) {
    let face_names = [
        top_candidates(block),
        side_candidates(block),
        bottom_candidates(block),
    ];

    let mut extracted = 0;
    let mut tiles =
        face_names.map(
            |candidates| match read_texture(assets_dir, &candidates, data) {
                Some(texture) => {
                    extracted += 1;
                    texture
                }
                None => {
                    eprintln!("{}: {candidates:?} not found - fallback color.", block.name);
                    fallback_tile(block, data.assets.tile_size)
                }
            },
        );

    if block.grass_tint {
        tint_grass(&mut tiles[0], &data.assets.grass_tint);
        let overlay_name = [format!("{}_side_overlay", block.name)];
        if let Some(mut overlay) = read_texture(assets_dir, &overlay_name, data) {
            tint_grass(&mut overlay, &data.assets.grass_tint);
            image::imageops::overlay(&mut tiles[1], &overlay, 0, 0);
        }
    }

    (tiles, extracted)
}

fn top_candidates(block: &BlockDefinition) -> Vec<String> {
    vec![format!("{}_top", block.name), block.name.to_string()]
}

fn side_candidates(block: &BlockDefinition) -> Vec<String> {
    vec![format!("{}_side", block.name), block.name.to_string()]
}

fn bottom_candidates(block: &BlockDefinition) -> Vec<String> {
    match block.bottom {
        Some(ref explicit) => vec![explicit.clone()],
        None => vec![
            format!("{}_bottom", block.name),
            format!("{}_top", block.name),
            block.name.to_string(),
        ],
    }
}

fn read_texture(assets_dir: &Path, names: &[String], data: &GameData) -> Option<image::RgbaImage> {
    for name in names {
        let path = assets_dir
            .join(&data.assets.texture_dir)
            .join(format!("{name}.png"));
        if let Ok(bytes) = fs::read(path)
            && let Ok(texture) = image::load_from_memory(&bytes)
        {
            return Some(texture.to_rgba8());
        }
    }
    None
}

fn tint_grass(texture: &mut image::RgbaImage, tint: &[u8; 3]) {
    for pixel in texture.pixels_mut() {
        for (channel, value) in tint.iter().enumerate() {
            pixel.0[channel] = (pixel.0[channel] as u16 * *value as u16 / 255) as u8;
        }
    }
}

fn fallback_tile(block: &BlockDefinition, tile_size: u32) -> image::RgbaImage {
    let [r, g, b] = block.fallback;
    image::RgbaImage::from_pixel(tile_size, tile_size, image::Rgba([r, g, b, 255]))
}

fn write_fallback_atlas(target: &Path, data: &GameData) {
    let texture_count = data.blocks.len() as u32 * data.assets.faces_per_block;
    let mut atlas =
        image::RgbaImage::new(data.assets.tile_size, data.assets.tile_size * texture_count);

    for (index, block) in data.blocks.iter().enumerate() {
        let base = index as u32 * data.assets.faces_per_block;
        let mut top = fallback_tile(block, data.assets.tile_size);
        let mut side = fallback_tile(block, data.assets.tile_size);
        let bottom = fallback_tile(block, data.assets.tile_size);
        if block.grass_tint {
            let [red, green, blue] = data.assets.grass_tint;
            let green = image::Rgba([red, green, blue, 255]);
            for pixel in top.pixels_mut() {
                *pixel = green;
            }
            for y in 0..data.assets.grass_overlay_height {
                for x in 0..data.assets.tile_size {
                    side.put_pixel(x, y, green);
                }
            }
        }
        for (offset, tile) in [top, side, bottom].into_iter().enumerate() {
            image::imageops::overlay(
                &mut atlas,
                &tile,
                0,
                ((base + offset as u32) * data.assets.tile_size) as i64,
            );
        }
    }

    if let Some(parent) = target.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = atlas.save(target);
}
