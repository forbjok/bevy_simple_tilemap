use std::sync::Mutex;

use bevy::{
    prelude::*,
    render::{
        camera::{ActiveCameras, Camera},
        draw::OutsideFrustum,
        mesh::Indices,
        pipeline::PrimitiveTopology,
    },
    tasks::AsyncComputeTaskPool,
    utils::HashMap,
};

use crate::bundle::ChunkBundle;

#[derive(Debug, Default)]
pub struct Chunk {
    chunk_size: IVec2,
    origin: IVec3,
    tiles: Vec<Option<Tile>>,
    needs_remesh: bool,
    size_in_pixels: Vec2,
}

#[derive(Clone, Debug, Default)]
pub struct Tile {
    pub sprite_index: u32,
    pub color: Color,
}

#[derive(Default)]
pub struct TileMap {
    chunk_size: IVec2,
    chunks: HashMap<IVec3, Entity>,
    tile_changes: Vec<(IVec3, Option<Tile>)>,
}

#[derive(Default)]
pub struct TileMapCache {
    tile_changes_by_chunk: HashMap<IVec3, Vec<(IVec3, Option<Tile>)>>,
}

impl Chunk {
    pub fn new(origin: IVec3, chunk_size: UVec2) -> Self {
        Self {
            chunk_size: chunk_size.as_i32(),
            origin,
            tiles: vec![None; (chunk_size.x * chunk_size.y) as usize],
            ..Default::default()
        }
    }
}

impl TileMap {
    pub fn new() -> Self {
        Self {
            chunk_size: IVec2::new(64, 64),
            chunks: Default::default(),
            tile_changes: Default::default(),
        }
    }

    pub fn set_tile(&mut self, pos: IVec3, tile: Option<Tile>) {
        self.tile_changes.push((pos, tile));
    }

    pub fn set_tiles(&mut self, tiles: impl IntoIterator<Item = (IVec3, Option<Tile>)>) {
        self.tile_changes
            .extend(tiles.into_iter().map(|(pos, tile)| (pos, tile)));
    }
}

/// Calculate chunk position based on tile position
fn calc_chunk_pos(tile_pos: IVec3, chunk_size: IVec2) -> IVec3 {
    IVec3::new(
        tile_pos.x.div_euclid(chunk_size.x),
        tile_pos.y.div_euclid(chunk_size.y),
        tile_pos.z,
    )
}

/// Calculate chunk origin (bottom left corner of chunk) in tile coordinates
fn calc_chunk_origin(chunk_pos: IVec3, chunk_size: IVec2) -> IVec3 {
    IVec3::new(chunk_pos.x * chunk_size.x, chunk_pos.y * chunk_size.y, chunk_pos.z)
}

/// Calculate row major index of tile position
fn row_major_index(pos: IVec2, chunk_size: IVec2) -> usize {
    ((pos.x * chunk_size.x) + pos.y) as usize
}

/// Calculate row major position from index
fn row_major_pos(index: usize, chunk_size: IVec2) -> IVec2 {
    IVec2::new(
        (index / chunk_size.x as usize) as i32,
        (index % chunk_size.y as usize) as i32,
    )
}

/// Update and mark chunks for remeshing, based on queued tile changes
pub(crate) fn update_chunks_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tilemap_query: Query<(Entity, &mut TileMap, &mut TileMapCache, &Handle<TextureAtlas>)>,
    mut chunk_query: Query<&mut Chunk>,
    texture_atlases: Res<Assets<TextureAtlas>>,
) {
    //let update_chunk_time = Instant::now();

    for (tilemap_entity, mut tilemap, mut tilemap_cache, texture_atlas_handle) in tilemap_query.iter_mut() {
        let chunk_size = tilemap.chunk_size;

        // Temporary storage for tile changes grouped by chunk
        let changes_by_chunk = &mut tilemap_cache.tile_changes_by_chunk;

        for (pos, tile) in tilemap.tile_changes.drain(..) {
            let chunk_pos = calc_chunk_pos(pos, chunk_size);

            changes_by_chunk
                .entry(chunk_pos)
                .or_insert_with(|| Vec::with_capacity((chunk_size.x * chunk_size.y) as usize))
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
                    let chunk_origin = chunk.origin;

                    for (pos, tile) in tiles.drain(..) {
                        let pos = pos - chunk_origin;
                        let index = row_major_index(pos.into(), chunk_size);

                        chunk.tiles[index] = tile;
                    }

                    // Mark chunk for remesh
                    chunk.needs_remesh = true;
                }
            } else {
                // Chunk does not exist yet, and needs to be spawned...

                let chunk_origin = calc_chunk_origin(*chunk_pos, chunk_size);

                let mut chunk = Chunk::new(chunk_origin, chunk_size.as_u32());
                chunk.needs_remesh = true;

                for (pos, tile) in tiles.drain(..) {
                    let pos = pos - chunk_origin;
                    let index = row_major_index(pos.into(), chunk_size);

                    chunk.tiles[index] = tile;
                }

                // Determine tile size in pixels from first sprite in TextureAtlas.
                // It is assumed and mandated that all sprites in the sprite sheet are the same size.
                let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
                let tile0_tex = texture_atlas.textures.get(0).unwrap();
                let tile_size = Vec2::new(tile0_tex.width(), tile0_tex.height());

                chunk.size_in_pixels = chunk_size.as_f32() * tile_size;

                // Calculate chunk translation
                let chunk_translation = (chunk_origin.truncate().as_f32() * tile_size).extend(chunk_origin.z as f32);

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
    mut chunk_query: Query<(&mut Chunk, &Handle<Mesh>, &Visible), Without<OutsideFrustum>>,
    meshes: ResMut<Assets<Mesh>>,
    task_pool: Res<AsyncComputeTaskPool>,
) {
    const VERTICES_PER_TILE: usize = 4;
    const INDICES_PER_TILE: usize = 6;

    let meshes = Mutex::new(meshes);

    //let remesh_time = Instant::now();

    chunk_query.par_for_each_mut(&task_pool, 8, |(mut chunk, mesh_handle, visible)| {
        if !chunk.needs_remesh || !visible.is_visible {
            return;
        }

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
            let pos = row_major_pos(i, chunk.chunk_size).as_f32();

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
        pub fn is_intersecting(&self, other: Rect) -> bool {
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

    for active_camera_entity in active_cameras.iter().filter_map(|a| a.entity) {
        if let Ok(camera_transform) = camera_transform_query.get(active_camera_entity) {
            let camera_size = window_size * camera_transform.scale.truncate();

            let camera_rect = Rect {
                anchor: Anchor::Center,
                position: camera_transform.translation.truncate(),
                size: camera_size,
            };

            for (entity, chunk_transform, chunk) in chunk_query.iter() {
                let size = chunk.size_in_pixels * chunk_transform.scale.truncate();

                let chunk_rect = Rect {
                    anchor: Anchor::BottomLeft,
                    position: chunk_transform.translation.truncate(),
                    size,
                };

                if camera_rect.is_intersecting(chunk_rect) {
                    if chunk_outside_frustum_query.get(entity).is_ok() {
                        commands.entity(entity).remove::<OutsideFrustum>();
                    }
                } else if chunk_outside_frustum_query.get(entity).is_err() {
                    commands.entity(entity).insert(OutsideFrustum);
                }
            }
        }
    }
}
