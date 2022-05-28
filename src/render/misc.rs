use bevy::asset::{AssetEvent, Assets};
use bevy::ecs::prelude::*;
use bevy::render::{render_resource::TextureUsages, texture::Image};

/// Set texture usages required by TextureArrayCache for newly loaded textures
pub fn set_texture_usages_system(
    mut texture_events: EventReader<AssetEvent<Image>>,
    mut textures: ResMut<Assets<Image>>,
) {
    for event in texture_events.iter() {
        if let AssetEvent::Created { handle } = event {
            if let Some(mut texture) = textures.get_mut(handle) {
                texture.texture_descriptor.usage =
                    TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC | TextureUsages::COPY_DST;
            }
        }
    }
}
