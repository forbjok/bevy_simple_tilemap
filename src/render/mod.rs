use std::ops::Range;

use bevy::{
    color::LinearRgba,
    math::{IVec2, IVec3, Mat4, URect, UVec2, Vec2},
    prelude::{AssetEvent, AssetId, Component, Entity, GlobalTransform, Handle, Image, Resource, Shader},
    render::render_resource::{BindGroup, BufferUsages, DynamicUniformBuffer, RawBufferVec, ShaderType},
    utils::HashMap,
};
use bytemuck::{Pod, Zeroable};

use crate::TileFlags;

pub mod draw;
pub mod extract;
pub mod pipeline;
pub mod queue;

pub const TILEMAP_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(9765236402292098257);

pub struct ExtractedTile {
    pub pos: IVec2,
    pub rect: URect,
    pub color: LinearRgba,
    pub flags: TileFlags,
}

pub struct ExtractedChunk {
    pub origin: IVec3,
    pub tiles: Vec<ExtractedTile>,
}

pub struct ExtractedTilemap {
    pub entity: Entity,
    pub transform: GlobalTransform,
    pub image_handle_id: AssetId<Image>,
    pub tile_size: UVec2,
    pub chunks: Vec<ExtractedChunk>,
    pub visible_chunks: Vec<IVec3>,
}

#[derive(Default, Resource)]
pub struct ExtractedTilemaps {
    pub tilemaps: Vec<ExtractedTilemap>,
}

#[derive(Default, Resource)]
pub struct TilemapAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct TilemapVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub tile_uv: [f32; 2],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable, ShaderType)]
pub struct TilemapGpuData {
    pub transform: Mat4,
    pub tile_size: Vec2,
    pub texture_size: Vec2,
}

pub struct ChunkMeta {
    vertices: RawBufferVec<TilemapVertex>,
    tilemap_gpu_data: DynamicUniformBuffer<TilemapGpuData>,
    tilemap_gpu_data_bind_group: Option<BindGroup>,
    texture_size: UVec2,
    tile_size: UVec2,
}

impl Default for ChunkMeta {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            tilemap_gpu_data: DynamicUniformBuffer::default(),
            tilemap_gpu_data_bind_group: None,
            texture_size: UVec2::ZERO,
            tile_size: UVec2::ZERO,
        }
    }
}

pub type ChunkKey = (Entity, IVec3);

#[derive(Default, Resource)]
pub struct TilemapMeta {
    chunks: HashMap<ChunkKey, ChunkMeta>,
    view_bind_group: Option<BindGroup>,
}

#[derive(Component, PartialEq, Clone, Eq)]
pub struct TilemapBatch {
    image_handle_id: AssetId<Image>,
    range: Range<u32>,
    chunk_key: (Entity, IVec3),
}

#[derive(Default, Resource)]
pub struct ImageBindGroups {
    values: HashMap<AssetId<Image>, BindGroup>,
}
