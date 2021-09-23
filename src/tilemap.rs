use bitflags::bitflags;
use std::sync::Mutex;

use bevy::{
    core::{Pod, Zeroable},
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::{ActiveCameras, Camera},
        draw::OutsideFrustum,
        mesh::Indices,
        pipeline::PrimitiveTopology,
        renderer::RenderResources,
    },
    tasks::AsyncComputeTaskPool,
    utils::{HashMap, HashSet},
};

use crate::bundle::ChunkBundle;

const CHUNK_WIDTH: u32 = 64;
const CHUNK_HEIGHT: u32 = 64;
const CHUNK_WIDTH_I32: i32 = CHUNK_WIDTH as i32;
const CHUNK_HEIGHT_I32: i32 = CHUNK_HEIGHT as i32;
const CHUNK_WIDTH_USIZE: usize = CHUNK_WIDTH as usize;

const TILES_PER_CHUNK: usize = (CHUNK_WIDTH * CHUNK_HEIGHT) as usize;

#[derive(Debug, Default)]
pub struct Chunk {
    origin: IVec3,
    tiles: Vec<Option<Tile>>,
    needs_remesh: bool,
    size_in_pixels: Vec2,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TileGpuData {
    pub sprite_index: u32,
    pub color: Vec4,
    pub flags: u32,
}

unsafe impl Zeroable for TileGpuData {}
unsafe impl Pod for TileGpuData {}

#[derive(Debug, Default, RenderResources, TypeUuid)]
#[uuid = "54d394ec-459c-48d3-9562-35fce6e88bda"]
pub struct ChunkGpuData {
    #[render_resources(buffer)]
    pub tiles: Vec<TileGpuData>,
}

bitflags! {
    #[derive(Default)]
    pub struct TileFlags: u32 {
        const FLIP_X = 1 << 0;
        const FLIP_Y = 1 << 1;
    }
}

#[derive(Clone, Debug, Default)]
pub struct Tile {
    pub sprite_index: u32,
    pub color: Color,
    pub flags: TileFlags,
}

#[derive(Default)]
pub struct TileMap {
    chunks: HashMap<IVec3, Entity>,
    tile_changes: Vec<(IVec3, Option<Tile>)>,
    clear_all: bool,
    clear_layers: HashSet<i32>,
}

#[derive(Default)]
pub struct TileMapCache {
    tile_changes_by_chunk: HashMap<IVec3, Vec<(IVec3, Option<Tile>)>>,
}

impl Chunk {
    pub fn new(origin: IVec3) -> Self {
        Self {
            origin,
            tiles: vec![None; (CHUNK_WIDTH * CHUNK_HEIGHT) as usize],
            ..Default::default()
        }
    }

    fn clear(&mut self) {
        for tile in self.tiles.iter_mut() {
            *tile = None;
        }

        self.needs_remesh = true;
    }

    fn set_tiles(&mut self, tiles: impl IntoIterator<Item = (IVec3, Option<Tile>)>) {
        let chunk_origin = self.origin;

        for (pos, tile) in tiles {
            let pos = pos - chunk_origin;
            let index = row_major_index(pos.into());

            self.tiles[index] = tile;
        }
    }
}

impl TileMap {
    pub fn clear(&mut self) {
        // Clear change queue
        self.tile_changes.clear();

        // Clear layer clear requests, since we're clearing everything anyway
        self.clear_layers.clear();

        // Request full clear
        self.clear_all = true;
    }

    pub fn clear_layer(&mut self, layer: i32) {
        // Remove queued tile changes for the cleared layer
        self.tile_changes.retain(|(pos, _)| pos.z != layer);

        // Request clear layer
        self.clear_layers.insert(layer);
    }

    pub fn set_tile(&mut self, pos: IVec3, tile: Option<Tile>) {
        self.tile_changes.push((pos, tile));
    }

    pub fn set_tiles(&mut self, tiles: impl IntoIterator<Item = (IVec3, Option<Tile>)>) {
        self.tile_changes
            .extend(tiles.into_iter().map(|(pos, tile)| (pos, tile)));
    }
}

impl From<&Tile> for TileGpuData {
    fn from(tile: &Tile) -> Self {
        Self {
            sprite_index: tile.sprite_index,
            color: tile.color.into(),
            flags: tile.flags.bits(),
        }
    }
}

/// Calculate chunk position based on tile position
#[inline]
fn calc_chunk_pos(tile_pos: IVec3) -> IVec3 {
    IVec3::new(
        tile_pos.x.div_euclid(CHUNK_WIDTH_I32),
        tile_pos.y.div_euclid(CHUNK_HEIGHT_I32),
        tile_pos.z,
    )
}

/// Calculate chunk origin (bottom left corner of chunk) in tile coordinates
#[inline]
fn calc_chunk_origin(chunk_pos: IVec3) -> IVec3 {
    IVec3::new(
        chunk_pos.x * CHUNK_WIDTH_I32,
        chunk_pos.y * CHUNK_HEIGHT_I32,
        chunk_pos.z,
    )
}

/// Calculate row major index of tile position
#[inline]
fn row_major_index(pos: IVec2) -> usize {
    (pos.x + pos.y * CHUNK_HEIGHT_I32) as usize
}

/// Calculate row major position from index
#[inline]
fn row_major_pos(index: usize) -> IVec2 {
    let y = index / CHUNK_WIDTH_USIZE;

    IVec2::new((index - (y * CHUNK_WIDTH_USIZE as usize)) as i32, y as i32)
}

/// Update and mark chunks for remeshing, based on queued tile changes
pub(crate) fn update_chunks_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut chunk_gpu_datas: ResMut<Assets<ChunkGpuData>>,
    mut tilemap_query: Query<(Entity, &mut TileMap, &mut TileMapCache, &Handle<TextureAtlas>)>,
    mut chunk_query: Query<&mut Chunk>,
    texture_atlases: Res<Assets<TextureAtlas>>,
) {
    //let update_chunk_time = Instant::now();

    for (tilemap_entity, mut tilemap, mut tilemap_cache, texture_atlas_handle) in tilemap_query.iter_mut() {
        // Temporary storage for tile changes grouped by chunk
        let changes_by_chunk = &mut tilemap_cache.tile_changes_by_chunk;

        // A full clear was requested. Clear all chunks.
        if tilemap.clear_all {
            for chunk_entity in tilemap.chunks.values() {
                if let Ok(mut chunk) = chunk_query.get_mut(*chunk_entity) {
                    chunk.clear();
                }
            }

            tilemap.clear_all = false;
        }

        if !tilemap.clear_layers.is_empty() {
            let clear_layers: Vec<i32> = tilemap.clear_layers.drain().collect();

            // Process clear layer requests
            for layer in clear_layers.into_iter() {
                for (_, chunk_entity) in tilemap.chunks.iter().filter(|(pos, _)| pos.z == layer) {
                    if let Ok(mut chunk) = chunk_query.get_mut(*chunk_entity) {
                        chunk.clear();
                    }
                }
            }
        }

        for (pos, tile) in tilemap.tile_changes.drain(..) {
            let chunk_pos = calc_chunk_pos(pos);

            changes_by_chunk
                .entry(chunk_pos)
                .or_insert_with(|| Vec::with_capacity(TILES_PER_CHUNK))
                .push((pos, tile));
        }

        // Apply tile changes for each chunk
        for (chunk_pos, tiles) in changes_by_chunk.iter_mut() {
            if tiles.is_empty() {
                continue;
            }

            if let Some(chunk_entity) = tilemap.chunks.get(&chunk_pos) {
                // Chunk already exists...
                if let Ok(mut chunk) = chunk_query.get_mut(*chunk_entity) {
                    // Set tiles in chunk
                    chunk.set_tiles(tiles.drain(..));

                    // Mark chunk for remesh
                    chunk.needs_remesh = true;
                }
            } else {
                // Chunk does not exist yet, and needs to be spawned...

                let chunk_origin = calc_chunk_origin(*chunk_pos);

                let mut chunk = Chunk::new(chunk_origin);
                chunk.needs_remesh = true;

                // Set tiles in chunk
                chunk.set_tiles(tiles.drain(..));

                let chunk_gpu_data = ChunkGpuData::default();
                let chunk_gpu_data = chunk_gpu_datas.add(chunk_gpu_data);

                // Determine tile size in pixels from first sprite in TextureAtlas.
                // It is assumed and mandated that all sprites in the sprite sheet are the same size.
                let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
                let tile0_tex = texture_atlas.textures.get(0).unwrap();
                let tile_size = Vec2::new(tile0_tex.width(), tile0_tex.height());

                chunk.size_in_pixels = Vec2::new(CHUNK_WIDTH as f32, CHUNK_HEIGHT as f32) * tile_size;

                // Calculate chunk translation
                let chunk_translation = (chunk_origin.truncate().as_vec2() * tile_size).extend(chunk_origin.z as f32);

                // Create new mesh for chunk
                let mesh = Mesh::new(PrimitiveTopology::TriangleList);
                let mesh = meshes.add(mesh);

                // Spawn chunk entity
                let chunk_entity = commands
                    .spawn()
                    .insert_bundle(ChunkBundle {
                        chunk,
                        chunk_gpu_data,
                        texture_atlas: texture_atlas_handle.clone(),
                        mesh,
                        transform: Transform::from_translation(chunk_translation),
                        ..Default::default()
                    })
                    .id();

                // Make chunk entity a child of the tilemap.
                // We use .push_children() for this, because simply inserting a Parent component
                // appears to be buggy and does not properly update transforms upon insertion.
                commands.entity(tilemap_entity).push_children(&[chunk_entity]);

                // Store chunk entity in the tilemap
                tilemap.chunks.insert(*chunk_pos, chunk_entity);
            }
        }
    }

    //dbg!(update_chunk_time.elapsed());
}

/// Remesh changed chunks
pub(crate) fn remesh_chunks_system(
    mut chunk_query: Query<(&mut Chunk, &Handle<ChunkGpuData>, &Handle<Mesh>, &Visible), Without<OutsideFrustum>>,
    chunk_gpu_datas: ResMut<Assets<ChunkGpuData>>,
    meshes: ResMut<Assets<Mesh>>,
    task_pool: Res<AsyncComputeTaskPool>,
) {
    const VERTICES_PER_TILE: usize = 4;
    const INDICES_PER_TILE: usize = 6;

    let chunk_gpu_datas = Mutex::new(chunk_gpu_datas);
    let meshes = Mutex::new(meshes);

    //let remesh_time = Instant::now();

    chunk_query.par_for_each_mut(
        &task_pool,
        8,
        |(mut chunk, chunk_gpu_data_handle, mesh_handle, visible)| {
            if !chunk.needs_remesh || !visible.is_visible {
                return;
            }

            let tile_count = chunk.tiles.len();

            let mut positions: Vec<[f32; 2]> = Vec::with_capacity(VERTICES_PER_TILE * tile_count);
            let mut indices: Vec<u32> = Vec::with_capacity(INDICES_PER_TILE * tile_count);
            let mut index: u32 = 0;

            let mut tiles: Vec<TileGpuData> = Vec::with_capacity(tile_count);

            for (i, tile) in chunk
                .tiles
                .iter()
                .enumerate()
                .filter_map(|(i, t)| t.as_ref().map(|t| (i, t)))
            {
                tiles.push(tile.into());

                // Calculate position in chunk based on tile index
                let pos = row_major_pos(i).as_vec2();

                positions.extend(
                    [
                        [pos.x, pos.y],
                        [pos.x, pos.y + 1.0],
                        [pos.x + 1.0, pos.y + 1.0],
                        [pos.x + 1.0, pos.y],
                    ]
                    .iter(),
                );

                indices.extend([index, index + 2, index + 1, index, index + 3, index + 2].iter());

                index += 4;
            }

            // If there are no tiles to render, add a default TileGpuData.
            // Workaround for "Size of the buffer needs to be greater than 0!" error.
            if tiles.is_empty() {
                tiles.push(Default::default());
            }

            let mut chunk_gpu_datas = chunk_gpu_datas.lock().unwrap();
            if let Some(chunk_gpu_data) = chunk_gpu_datas.get_mut(chunk_gpu_data_handle) {
                *chunk_gpu_data = ChunkGpuData { tiles };
            }

            let mut new_mesh = Mesh::new(PrimitiveTopology::TriangleList);
            new_mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            new_mesh.set_indices(Some(Indices::U32(indices)));

            let mut meshes = meshes.lock().unwrap();
            if let Some(mesh) = meshes.get_mut(mesh_handle) {
                *mesh = new_mesh;
            }

            chunk.needs_remesh = false;
        },
    );

    //dbg!(remesh_time.elapsed());
}

/// Propagate TileMap visibility to chunks
pub(crate) fn propagate_visibility_system(
    mut tilemap_query: Query<(&TileMap, &Visible), (Changed<Visible>, With<TileMap>)>,
    mut chunk_query: Query<&mut Visible, (With<Chunk>, Without<TileMap>)>,
) {
    for (tilemap, tilemap_visible) in tilemap_query.iter_mut() {
        for chunk_entity in tilemap.chunks.values() {
            if let Ok(mut chunk_visible) = chunk_query.get_mut(*chunk_entity) {
                chunk_visible.is_visible = tilemap_visible.is_visible;
            }
        }
    }
}

/// Perform frustum culling of chunks
pub(crate) fn tilemap_frustum_culling_system(
    mut commands: Commands,
    windows: Res<Windows>,
    active_cameras: Res<ActiveCameras>,
    camera_transform_query: Query<&GlobalTransform, With<Camera>>,
    chunk_outside_frustum_query: Query<&OutsideFrustum, With<Chunk>>,
    chunk_query: Query<(Entity, &GlobalTransform, &Chunk)>,
) {
    enum Anchor {
        BottomLeft,
        Center,
    }

    struct Rect {
        anchor: Anchor,
        position: Vec2,
        size: Vec2,
    }

    impl Rect {
        #[inline]
        pub fn is_intersecting(&self, other: &Rect) -> bool {
            self.get_center_position().distance(other.get_center_position()) < (self.get_radius() + other.get_radius())
        }

        #[inline]
        pub fn get_center_position(&self) -> Vec2 {
            match self.anchor {
                Anchor::BottomLeft => self.position + (self.size / 2.0),
                Anchor::Center => self.position,
            }
        }

        #[inline]
        pub fn get_radius(&self) -> f32 {
            let half_size = self.size / Vec2::splat(2.0);
            (half_size.x.powf(2.0) + half_size.y.powf(2.0)).sqrt()
        }
    }

    let window = windows.get_primary().unwrap();
    let window_size = Vec2::new(window.width(), window.height());

    let camera_rects = {
        let mut camera_rects: Vec<Rect> = Vec::with_capacity(3);

        for active_camera_entity in active_cameras.iter().filter_map(|a| a.entity) {
            if let Ok(camera_transform) = camera_transform_query.get(active_camera_entity) {
                let camera_size = window_size * camera_transform.scale.truncate();

                let camera_rect = Rect {
                    anchor: Anchor::Center,
                    position: camera_transform.translation.truncate(),
                    size: camera_size,
                };

                camera_rects.push(camera_rect);
            }
        }

        camera_rects
    };

    for (entity, chunk_transform, chunk) in chunk_query.iter() {
        let size = chunk.size_in_pixels * chunk_transform.scale.truncate();

        let chunk_rect = Rect {
            anchor: Anchor::BottomLeft,
            position: chunk_transform.translation.truncate(),
            size,
        };

        if camera_rects.iter().any(|cr| cr.is_intersecting(&chunk_rect)) {
            if chunk_outside_frustum_query.get(entity).is_ok() {
                commands.entity(entity).remove::<OutsideFrustum>();
            }
        } else if chunk_outside_frustum_query.get(entity).is_err() {
            commands.entity(entity).insert(OutsideFrustum);
        }
    }
}
