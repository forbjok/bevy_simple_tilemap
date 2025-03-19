use std::time::Instant;

use bitflags::bitflags;

use bevy::{
    platform_support::collections::{HashMap, HashSet},
    prelude::*,
    render::{
        sync_world::SyncToRenderWorld,
        view::{self, VisibilityClass},
    },
};

pub(crate) const CHUNK_WIDTH: u32 = 64;
pub(crate) const CHUNK_HEIGHT: u32 = 64;
const CHUNK_WIDTH_I32: i32 = CHUNK_WIDTH as i32;
const CHUNK_HEIGHT_I32: i32 = CHUNK_HEIGHT as i32;
const CHUNK_WIDTH_USIZE: usize = CHUNK_WIDTH as usize;

const TILES_PER_CHUNK: usize = (CHUNK_WIDTH * CHUNK_HEIGHT) as usize;

#[derive(Clone, Debug)]
pub struct Chunk {
    pub origin: IVec3,
    pub tiles: Vec<Option<Tile>>,
    pub last_change_at: Instant,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default)]
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

#[derive(Component, Debug)]
#[require(TileMapCache, Transform, Visibility, SyncToRenderWorld, VisibilityClass)]
#[component(on_add = view::add_visibility_class::<Sprite>)]
pub struct TileMap {
    pub image: Handle<Image>,
    pub texture_atlas_layout: Handle<TextureAtlasLayout>,

    pub chunks: HashMap<IVec3, Chunk>,

    tile_changes: Vec<(IVec3, Option<Tile>)>,
    clear_all: bool,
    clear_layers: HashSet<i32>,
}

#[derive(Component, Default)]
pub struct TileMapCache {
    tile_changes_by_chunk: HashMap<IVec3, Vec<(IVec3, Option<Tile>)>>,
}

impl Chunk {
    pub fn new(origin: IVec3) -> Self {
        Self {
            origin,
            tiles: vec![None; (CHUNK_WIDTH * CHUNK_HEIGHT) as usize],
            last_change_at: Instant::now(),
        }
    }

    fn clear(&mut self) {
        for tile in self.tiles.iter_mut() {
            *tile = None;
        }

        self.last_change_at = Instant::now();
    }

    fn set_tiles(&mut self, tiles: impl IntoIterator<Item = (IVec3, Option<Tile>)>) {
        let chunk_origin = self.origin;

        for (pos, tile) in tiles {
            let pos = pos - chunk_origin;
            let index = row_major_index(IVec2::new(pos.x, pos.y));

            self.tiles[index] = tile;
        }

        self.last_change_at = Instant::now();
    }
}

impl TileMap {
    pub fn new(image: Handle<Image>, texture_atlas_layout: Handle<TextureAtlasLayout>) -> Self {
        Self {
            image,
            texture_atlas_layout,

            chunks: Default::default(),
            tile_changes: Default::default(),
            clear_all: false,
            clear_layers: Default::default(),
        }
    }

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
        self.tile_changes.extend(tiles);
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
pub fn row_major_pos(index: usize) -> IVec2 {
    let y = index / CHUNK_WIDTH_USIZE;

    IVec2::new((index - (y * CHUNK_WIDTH_USIZE)) as i32, y as i32)
}

/// Update and mark chunks for remeshing, based on queued tile changes
pub(crate) fn update_chunks_system(mut tilemap_query: Query<(&mut TileMap, &mut TileMapCache)>) {
    for (mut tilemap, mut tilemap_cache) in tilemap_query.iter_mut() {
        // Temporary storage for tile changes grouped by chunk
        let changes_by_chunk = &mut tilemap_cache.tile_changes_by_chunk;

        // A full clear was requested. Clear all chunks.
        if tilemap.clear_all {
            for chunk in tilemap.chunks.values_mut() {
                chunk.clear();
            }

            tilemap.clear_all = false;
        }

        if !tilemap.clear_layers.is_empty() {
            // Suppress bogus clippy warning.
            // We DO in fact need to collect here, as we otherwise get
            // a "cannot borrow `tilemap` as mutable more than once at a time"
            // within the loop.
            #[allow(clippy::needless_collect)]
            let clear_layers: Vec<i32> = tilemap.clear_layers.drain().collect();

            // Process clear layer requests
            for layer in clear_layers.into_iter() {
                for (_, chunk) in tilemap.chunks.iter_mut().filter(|(pos, _)| pos.z == layer) {
                    chunk.clear();
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

            match tilemap.chunks.get_mut(chunk_pos) { Some(chunk) => {
                // Chunk already exists...

                // Set tiles in chunk
                chunk.set_tiles(tiles.drain(..));
            } _ => {
                // Chunk does not exist yet, and needs to be spawned...

                let chunk_origin = calc_chunk_origin(*chunk_pos);

                let mut chunk = Chunk::new(chunk_origin);

                // Set tiles in chunk
                chunk.set_tiles(tiles.drain(..));

                // Store chunk entity in the tilemap
                tilemap.chunks.insert(*chunk_pos, chunk);
            }}
        }
    }
}
