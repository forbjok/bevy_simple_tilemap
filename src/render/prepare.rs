use bevy::{
    prelude::{Res, ResMut},
    render::{render_resource::FilterMode, renderer::RenderDevice},
};

use super::{texture_array_cache::TextureArrayCache, ExtractedTilemaps};

pub fn prepare_textures(
    render_device: Res<RenderDevice>,
    mut texture_array_cache: ResMut<TextureArrayCache>,
    extracted_tilemaps: Res<ExtractedTilemaps>,
) {
    for tilemap in extracted_tilemaps.tilemaps.iter() {
        texture_array_cache.add(
            &tilemap.texture,
            tilemap.tile_size,
            tilemap.texture_size,
            tilemap.padding,
            FilterMode::Nearest,
        );
    }

    texture_array_cache.prepare(&render_device);
}
