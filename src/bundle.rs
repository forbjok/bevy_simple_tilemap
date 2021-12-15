use bevy::{
    prelude::*,
};

use crate::{
    tilemap::{Chunk, ChunkGpuData, TileMap, TileMapCache},
};

#[derive(Bundle)]
pub struct TileMapBundle {
    pub tilemap: TileMap,
    pub tilemap_cache: TileMapCache,
    pub texture_atlas: Handle<TextureAtlas>,
    pub visibility: Visibility,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub computed_visibility: ComputedVisibility,
}

impl Default for TileMapBundle {
    fn default() -> Self {
        Self {
            tilemap: Default::default(),
            tilemap_cache: Default::default(),
            visibility: Default::default(),
            texture_atlas: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            computed_visibility: Default::default(),
        }
    }
}

#[derive(Bundle)]
pub(crate) struct ChunkBundle {
    pub chunk: Chunk,
    pub chunk_gpu_data: Handle<ChunkGpuData>,
    pub texture_atlas: Handle<TextureAtlas>,
    pub visibility: Visibility,
    pub mesh: Handle<Mesh>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for ChunkBundle {
    fn default() -> Self {
        Self {
            chunk: Default::default(),
            chunk_gpu_data: Default::default(),
            visibility: Default::default(),
            mesh: Default::default(),
            texture_atlas: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}
