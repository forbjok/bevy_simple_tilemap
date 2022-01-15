use bevy::{
    asset::HandleId,
    math::{IVec2, Vec2, IVec3},
    prelude::{AssetEvent, Color, Component, GlobalTransform, Handle, HandleUntyped, Image, Shader},
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
    pub color: u32,
}

/// Probably a cache of GPU data to be used in shaders?
pub struct TilemapMeta {
    vertices: BufferVec<TilemapVertex>,
    view_bind_group: Option<BindGroup>,
}

impl Default for TilemapMeta {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
            view_bind_group: None,
        }
    }
}

#[derive(Component, Eq, PartialEq, Copy, Clone)]
pub struct TilemapBatch {
    image_handle_id: HandleId,
}

#[derive(Default)]
pub struct ImageBindGroups {
    values: HashMap<Handle<Image>, BindGroup>,
}
