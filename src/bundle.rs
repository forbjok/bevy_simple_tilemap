use bevy::prelude::*;

use crate::tilemap::{TileMap, TileMapCache};

#[derive(Bundle, Default)]
pub struct TileMapBundle {
    pub tilemap: TileMap,
    pub tilemap_cache: TileMapCache,
    pub texture: Handle<Image>,
    pub atlas: TextureAtlas,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub view_visibility: ViewVisibility,
}
