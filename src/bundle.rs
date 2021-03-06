use bevy::prelude::*;

use crate::tilemap::{TileMap, TileMapCache};

#[derive(Bundle, Default)]
pub struct TileMapBundle {
    pub tilemap: TileMap,
    pub tilemap_cache: TileMapCache,
    pub texture_atlas: Handle<TextureAtlas>,
    pub visibility: Visibility,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub computed_visibility: ComputedVisibility,
}
