use bevy::asset::{AssetEvent, Assets, Handle};
use bevy::ecs::prelude::*;
use bevy::prelude::info;
use bevy::render::{texture::Image, view::ComputedVisibility, RenderWorld};
use bevy::sprite::TextureAtlas;
use bevy::transform::components::GlobalTransform;
use bevy::utils::Instant;

use crate::tilemap::row_major_pos;
use crate::TileMap;

use super::*;

pub fn extract_tilemap_events(mut render_world: ResMut<RenderWorld>, mut image_events: EventReader<AssetEvent<Image>>) {
    //let timer = Instant::now();

    let mut events = render_world.get_resource_mut::<TilemapAssetEvents>().unwrap();
    let TilemapAssetEvents { ref mut images } = *events;
    images.clear();

    for image in image_events.iter() {
        // AssetEvent: !Clone
        images.push(match image {
            AssetEvent::Created { handle } => AssetEvent::Created {
                handle: handle.clone_weak(),
            },
            AssetEvent::Modified { handle } => AssetEvent::Modified {
                handle: handle.clone_weak(),
            },
            AssetEvent::Removed { handle } => AssetEvent::Removed {
                handle: handle.clone_weak(),
            },
        });
    }

    //info!("EXT TM EVENTS {:?}", timer.elapsed());
}

pub fn extract_tilemaps(
    mut render_world: ResMut<RenderWorld>,
    images: Res<Assets<Image>>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    tilemap_query: Query<(&ComputedVisibility, &TileMap, &GlobalTransform, &Handle<TextureAtlas>)>,
) {
    let timer = Instant::now();

    let mut extracted_tilemaps = render_world.get_resource_mut::<ExtractedTilemaps>().unwrap();
    extracted_tilemaps.tilemaps.clear();
    for (computed_visibility, tilemap, transform, texture_atlas_handle) in tilemap_query.iter() {
        if !computed_visibility.is_visible {
            continue;
        }

        if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
            if images.contains(&texture_atlas.texture) {
                let mut chunks: Vec<ExtractedChunk> = Vec::with_capacity(tilemap.chunks.len());

                for (_pos, chunk) in tilemap.chunks.iter() {
                    let mut tiles: Vec<ExtractedTile> = Vec::with_capacity(chunk.tiles.len());

                    for (i, tile) in chunk.tiles.iter().enumerate() {
                        if let Some(tile) = tile {
                            let rect = texture_atlas.textures[tile.sprite_index as usize];

                            tiles.push(ExtractedTile {
                                pos: chunk.origin.truncate() + row_major_pos(i),
                                rect,
                                color: tile.color,
                                flags: tile.flags,
                            });
                        }
                    }

                    chunks.push(ExtractedChunk {
                        origin: chunk.origin,
                        tiles,
                    });
                }

                extracted_tilemaps.tilemaps.push(ExtractedTilemap {
                    transform: *transform,
                    image_handle_id: texture_atlas.texture.id,
                    atlas_size: texture_atlas.size,
                    chunks,
                });
            }
        }
    }
    info!("EXT TILEMAP {:?}", timer.elapsed());
}
