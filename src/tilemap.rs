use std::sync::Mutex;

use bevy::{
    prelude::*,
    render::{mesh::Indices, pipeline::PrimitiveTopology},
    tasks::AsyncComputeTaskPool,
    utils::HashMap,
};

use crate::bundle::ChunkBundle;

const CHUNK_WIDTH: u32 = 64;
const CHUNK_HEIGHT: u32 = 64;
const CHUNK_WIDTH_I32: i32 = CHUNK_WIDTH as i32;
const CHUNK_HEIGHT_I32: i32 = CHUNK_HEIGHT as i32;
const CHUNK_WIDTH_USIZE: usize = CHUNK_WIDTH as usize;
const CHUNK_HEIGHT_USIZE: usize = CHUNK_HEIGHT as usize;

const TILES_PER_CHUNK: usize = (CHUNK_WIDTH * CHUNK_HEIGHT) as usize;

#[derive(Debug, Default)]
pub struct Chunk {
    pub origin: IVec3,
    pub tiles: Vec<Option<Tile>>,
    pub needs_remesh: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Tile {
    pub sprite_index: u32,
    pub color: Color,
}

#[derive(Default)]
pub struct TileMap {
    pub tile_changes: Vec<(IVec3, Option<Tile>)>,
    pub chunks: HashMap<IVec3, Entity>,
}

#[derive(Default)]
pub struct TileMapCache {
    pub tile_changes_by_chunk: HashMap<IVec3, Vec<(IVec3, Option<Tile>)>>,
}

impl Chunk {
    pub fn new(origin: IVec3) -> Self {
        Self {
            origin,
            tiles: vec![None; (CHUNK_WIDTH * CHUNK_HEIGHT) as usize],
            ..Default::default()
        }
    }
}

impl TileMap {
    pub fn set_tiles(&mut self, tiles: impl IntoIterator<Item = (IVec3, Option<Tile>)>) {
        self.tile_changes
            .extend(tiles.into_iter().map(|(pos, tile)| (pos, tile)));
    }
}

/// Calculate chunk position based on tile position
fn calc_chunk_pos(tile_pos: IVec3) -> IVec3 {
    IVec3::new(
        tile_pos.x.div_euclid(CHUNK_WIDTH_I32),
        tile_pos.y.div_euclid(CHUNK_HEIGHT_I32),
        tile_pos.z,
    )
}

/// Calculate chunk origin (bottom left corner of chunk) in tile coordinates
fn calc_chunk_origin(chunk_pos: IVec3) -> IVec3 {
    IVec3::new(
        chunk_pos.x * CHUNK_WIDTH_I32,
        chunk_pos.y * CHUNK_HEIGHT_I32,
        chunk_pos.z,
    )
}

/// Calculate row major index of tile position
fn row_major_index(pos: IVec2) -> usize {
    ((pos.x * CHUNK_HEIGHT_I32) + pos.y) as usize
}

/// Calculate row major position from index
fn row_major_pos(index: usize) -> IVec2 {
    IVec2::new((index / CHUNK_WIDTH_USIZE) as i32, (index % CHUNK_HEIGHT_USIZE) as i32)
}

/// Update and mark chunks for remeshing, based on queued tile changes
pub(crate) fn update_chunk_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tilemap_query: Query<(Entity, &mut TileMap, &mut TileMapCache, &Handle<TextureAtlas>)>,
    mut chunk_query: Query<&mut Chunk>,
) {
    //let update_chunk_time = Instant::now();

    for (tilemap_entity, mut tilemap, mut tilemap_cache, texture_atlas_handle) in tilemap_query.iter_mut() {
        // Temporary storage for tile changes grouped by chunk
        let changes_by_chunk = &mut tilemap_cache.tile_changes_by_chunk;

        for (pos, tile) in tilemap.tile_changes.drain(..) {
            let chunk_pos = calc_chunk_pos(pos);

            changes_by_chunk
                .entry(chunk_pos)
                .or_insert_with(|| Vec::with_capacity(TILES_PER_CHUNK))
                .push((pos, tile));
        }

        // Apply tile changes for each chunk
        for (chunk_pos, tiles) in changes_by_chunk.iter_mut() {
            if let Some(chunk_entity) = tilemap.chunks.get(&chunk_pos) {
                // Chunk already exists...
                if let Ok(mut chunk) = chunk_query.get_mut(*chunk_entity) {
                    let chunk_origin = chunk.origin;

                    for (pos, tile) in tiles.drain(..) {
                        let pos = pos - chunk_origin;
                        let index = row_major_index(pos.into());

                        chunk.tiles[index] = tile;
                    }

                    // Mark chunk for remesh
                    chunk.needs_remesh = true;
                }
            } else {
                // Chunk does not exist yet, and needs to be spawned...

                let chunk_origin = calc_chunk_origin(*chunk_pos);

                let mut chunk = Chunk::new(chunk_origin);

                for (pos, tile) in tiles.drain(..) {
                    let pos = pos - chunk_origin;
                    let index = row_major_index(pos.into());

                    chunk.tiles[index] = tile;
                }

                chunk.needs_remesh = true;

                // Create new mesh for chunk
                let mesh = Mesh::new(PrimitiveTopology::TriangleList);
                let mesh = meshes.add(mesh);

                // Spawn chunk entity
                let chunk_entity = commands
                    .spawn()
                    .insert_bundle(ChunkBundle {
                        chunk,
                        texture_atlas: texture_atlas_handle.clone(),
                        mesh,
                        transform: Transform::from_translation(Vec3::new(0.0, 0.0, chunk_origin.z as f32)),
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
pub(crate) fn update_chunk_mesh_system(
    mut chunk_query: Query<(&mut Chunk, &Handle<Mesh>)>,
    meshes: ResMut<Assets<Mesh>>,
    task_pool: Res<AsyncComputeTaskPool>,
) {
    const VERTICES_PER_TILE: usize = 4;
    const INDICES_PER_TILE: usize = 6;

    let meshes = Mutex::new(meshes);

    //let remesh_time = Instant::now();

    chunk_query.par_for_each_mut(&task_pool, 8, |(mut chunk, mesh_handle)| {
        if !chunk.needs_remesh {
            return;
        }

        let origin = chunk.origin;
        let tile_count = chunk.tiles.len();

        let mut positions: Vec<[f32; 2]> = Vec::with_capacity(VERTICES_PER_TILE * tile_count);
        let mut sprite_indexes: Vec<u32> = Vec::with_capacity(VERTICES_PER_TILE * tile_count);
        let mut sprite_colors: Vec<[f32; 4]> = Vec::with_capacity(VERTICES_PER_TILE * tile_count);
        let mut indices: Vec<u32> = Vec::with_capacity(INDICES_PER_TILE * tile_count);
        let mut index: u32 = 0;

        for (i, tile) in chunk
            .tiles
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.as_ref().map(|t| (i, t)))
        {
            // Calculate position in chunk based on tile index
            let pos = origin + row_major_pos(i).extend(0);

            // Convert position to f32
            let pos = pos.as_f32();

            positions.extend(
                [
                    [pos.x, pos.y],
                    [pos.x, pos.y + 1.0],
                    [pos.x + 1.0, pos.y + 1.0],
                    [pos.x + 1.0, pos.y],
                ]
                .iter(),
            );

            sprite_indexes.extend(
                [
                    tile.sprite_index,
                    tile.sprite_index,
                    tile.sprite_index,
                    tile.sprite_index,
                ]
                .iter(),
            );

            let tile_color: [f32; 4] = tile.color.into();
            sprite_colors.extend([tile_color, tile_color, tile_color, tile_color].iter());

            indices.extend([index, index + 2, index + 1, index, index + 3, index + 2].iter());

            index += 4;
        }

        let mut new_mesh = Mesh::new(PrimitiveTopology::TriangleList);
        new_mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        new_mesh.set_attribute("SpriteIndex", sprite_indexes);
        new_mesh.set_attribute("SpriteColor", sprite_colors);
        new_mesh.set_indices(Some(Indices::U32(indices)));

        let mut meshes = meshes.lock().unwrap();
        if let Some(mesh) = meshes.get_mut(mesh_handle) {
            *mesh = new_mesh;
        }

        chunk.needs_remesh = false;
    });

    //dbg!(remesh_time.elapsed());
}
