use bevy::{
    prelude::*,
    render::{pipeline::RenderPipeline, render_graph::base::MainPass},
};

use crate::{
    render::TILEMAP_PIPELINE_HANDLE,
    tilemap::{Chunk, TileMap, TileMapCache},
};

#[derive(Bundle)]
pub struct TileMapBundle {
    pub tilemap: TileMap,
    pub tilemap_cache: TileMapCache,
    pub texture_atlas: Handle<TextureAtlas>,
    pub visible: Visible,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for TileMapBundle {
    fn default() -> Self {
        Self {
            tilemap: TileMap::new(),
            tilemap_cache: Default::default(),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            texture_atlas: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(Bundle)]
pub(crate) struct ChunkBundle {
    pub chunk: Chunk,
    pub texture_atlas: Handle<TextureAtlas>,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub main_pass: MainPass,
    pub mesh: Handle<Mesh>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for ChunkBundle {
    fn default() -> Self {
        Self {
            chunk: Chunk::default(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                TILEMAP_PIPELINE_HANDLE.typed(),
            )]),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            main_pass: MainPass,
            mesh: Default::default(),
            draw: Default::default(),
            texture_atlas: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}
