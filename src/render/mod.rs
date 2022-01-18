use bevy::{
    asset::HandleId,
    math::{IVec2, IVec3, Vec2},
    prelude::{AssetEvent, Color, Component, Entity, GlobalTransform, Handle, HandleUntyped, Image, Shader},
    reflect::TypeUuid,
    render::render_resource::{BindGroup, BufferUsages, BufferVec},
    sprite::Rect,
    utils::HashMap,
};
use bytemuck::{Pod, Zeroable};

use crate::TileFlags;

pub mod draw;
pub mod extract;
pub mod pipeline;
pub mod queue;

pub const TILEMAP_SHADER_HANDLE: HandleUntyped = HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 9765236402292098257);

pub struct ExtractedTile {
    pub pos: IVec2,
    pub rect: Rect,
    pub color: Color,
    pub flags: TileFlags,
}

pub struct ExtractedChunk {
    pub origin: IVec3,
    pub tiles: Vec<ExtractedTile>,
}

pub struct ExtractedTilemap {
    pub entity: Entity,
    pub transform: GlobalTransform,
    pub image_handle_id: HandleId,
    pub atlas_size: Vec2,
    pub chunks: Vec<ExtractedChunk>,
}

#[derive(Default)]
pub struct ExtractedTilemaps {
    pub tilemaps: Vec<ExtractedTilemap>,
}

#[derive(Default)]
pub struct TilemapAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct TilemapVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable)]
pub struct TileGpuData {
    pub color: u32,
}

pub struct ChunkMeta {
    vertices: BufferVec<TilemapVertex>,
    tile_gpu_datas: BufferVec<TileGpuData>,
    tile_gpu_data_bind_group: Option<BindGroup>,
}

impl Default for ChunkMeta {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
            tile_gpu_datas: BufferVec::new(BufferUsages::STORAGE),
            tile_gpu_data_bind_group: None,
        }
    }
}

pub type ChunkKey = (Entity, IVec3);

#[derive(Default)]
pub struct TilemapMeta {
    chunks: HashMap<ChunkKey, ChunkMeta>,
    view_bind_group: Option<BindGroup>,
}

#[derive(Component, Eq, PartialEq, Copy, Clone)]
pub struct TilemapBatch {
    image_handle_id: HandleId,
    chunk_key: (Entity, IVec3),
}

#[derive(Default)]
pub struct ImageBindGroups {
    values: HashMap<Handle<Image>, BindGroup>,
}
