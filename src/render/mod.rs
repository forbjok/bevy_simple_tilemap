use bevy::{
    asset::HandleId,
    math::{IVec2, IVec3, Mat4, UVec2},
    prelude::{AssetEvent, Color, Component, Entity, GlobalTransform, Handle, HandleUntyped, Image, Shader},
    reflect::TypeUuid,
    render::render_resource::{std140::AsStd140, BindGroup, BufferUsages, BufferVec, DynamicUniformVec},
    utils::{HashMap, Instant},
};
use bytemuck::{Pod, Zeroable};

use crate::TileFlags;

pub mod draw;
pub mod extract;
pub mod misc;
pub mod pipeline;
pub mod prepare;
pub mod queue;
pub mod texture_array_cache;

pub const TILEMAP_SHADER_HANDLE: HandleUntyped = HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 9765236402292098257);

pub struct ExtractedTile {
    pub pos: IVec2,
    pub index: u32,
    pub color: Color,
    pub flags: TileFlags,
}

pub struct ExtractedChunk {
    pub origin: IVec3,
    pub tiles: Vec<ExtractedTile>,
    pub last_change_at: Instant,
}

pub struct ExtractedTilemap {
    pub entity: Entity,
    pub tile_size: UVec2,
    pub texture_size: UVec2,
    pub padding: UVec2,
    pub transform: GlobalTransform,
    pub texture: Handle<Image>,
    pub chunks: Vec<ExtractedChunk>,
    pub visible_chunks: Vec<IVec3>,
}

#[derive(Default)]
pub struct ExtractedTilemaps {
    pub tilemaps: Vec<ExtractedTilemap>,
    pub chunks_changed_at: HashMap<ChunkKey, Instant>,
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
    pub sprite_index: i32,
    pub color: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable, AsStd140)]
pub struct TilemapGpuData {
    pub transform: Mat4,
}

pub struct ChunkMeta {
    vertices: BufferVec<TilemapVertex>,
    tilemap_gpu_data: DynamicUniformVec<TilemapGpuData>,
    tilemap_gpu_data_bind_group: Option<BindGroup>,
}

impl Default for ChunkMeta {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
            tilemap_gpu_data: DynamicUniformVec::default(),
            tilemap_gpu_data_bind_group: None,
        }
    }
}

pub type ChunkKey = (Entity, IVec3);

#[derive(Default)]
pub struct TilemapMeta {
    chunks: HashMap<ChunkKey, ChunkMeta>,
    view_bind_group: Option<BindGroup>,
}

#[derive(Component, PartialEq, Copy, Clone)]
pub struct TilemapBatch {
    image_handle_id: HandleId,
    chunk_key: (Entity, IVec3),
}

#[derive(Default)]
pub struct ImageBindGroups {
    values: HashMap<Handle<Image>, BindGroup>,
}
