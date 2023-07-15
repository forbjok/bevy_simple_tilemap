use std::ops::Mul;

use bevy::asset::{AssetEvent, Assets, Handle};
use bevy::ecs::prelude::*;
use bevy::prelude::*;
use bevy::render::Extract;
use bevy::render::{texture::Image, view::ComputedVisibility};
use bevy::sprite::TextureAtlas;
use bevy::transform::components::GlobalTransform;

#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::tilemap::{row_major_pos, CHUNK_HEIGHT, CHUNK_WIDTH};
use crate::TileMap;

use super::*;

pub fn extract_tilemap_events(
    mut events: ResMut<TilemapAssetEvents>,
    mut image_events: Extract<EventReader<AssetEvent<Image>>>,
) {
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
}

#[allow(clippy::type_complexity)]
pub fn extract_tilemaps(
    mut extracted_tilemaps: ResMut<ExtractedTilemaps>,
    images: Extract<Res<Assets<Image>>>,
    texture_atlases: Extract<Res<Assets<TextureAtlas>>>,
    tilemap_query: Extract<
        Query<(
            Entity,
            &ComputedVisibility,
            &TileMap,
            &GlobalTransform,
            &Handle<TextureAtlas>,
        )>,
    >,
    window_query: Extract<Query<&Window>>,
    camera_transform_query: Extract<Query<&GlobalTransform, With<Camera2d>>>,
) {
    enum Anchor {
        BottomLeft,
        Center,
    }

    struct Rect {
        anchor: Anchor,
        position: Vec2,
        size: Vec2,
    }

    impl Rect {
        #[inline]
        pub fn is_intersecting(&self, other: &Rect) -> bool {
            self.get_center_position().distance(other.get_center_position()) < (self.get_radius() + other.get_radius())
        }

        #[inline]
        pub fn get_center_position(&self) -> Vec2 {
            match self.anchor {
                Anchor::BottomLeft => self.position + (self.size / 2.0),
                Anchor::Center => self.position,
            }
        }

        #[inline]
        pub fn get_radius(&self) -> f32 {
            let half_size = self.size / Vec2::splat(2.0);
            (half_size.x.powf(2.0) + half_size.y.powf(2.0)).sqrt()
        }
    }

    let window = window_query.iter().next();
    if window.is_none() {
        return;
    }

    let window = window.unwrap();

    let window_size = Vec2::new(window.width(), window.height());

    let camera_rects = {
        let mut camera_rects: Vec<Rect> = Vec::with_capacity(3);

        for camera_transform in camera_transform_query.iter() {
            let (camera_scale, _, camera_translation) = camera_transform.to_scale_rotation_translation();
            let camera_size = window_size * camera_scale.truncate();

            let camera_rect = Rect {
                anchor: Anchor::Center,
                position: camera_translation.truncate(),
                size: camera_size,
            };

            camera_rects.push(camera_rect);
        }

        camera_rects
    };

    extracted_tilemaps.tilemaps.clear();

    for (entity, computed_visibility, tilemap, transform, texture_atlas_handle) in tilemap_query.iter() {
        if !computed_visibility.is_visible() {
            continue;
        }

        if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
            if images.contains(&texture_atlas.texture) {
                let (scale, _, _) = transform.to_scale_rotation_translation();

                // Determine tile size in pixels from first sprite in TextureAtlas.
                // It is assumed and mandated that all sprites in the sprite sheet are the same size.
                let tile0_tex = texture_atlas.textures.get(0).unwrap();
                let tile_size = Vec2::new(tile0_tex.width(), tile0_tex.height());

                let chunk_pixel_size = Vec2::new(CHUNK_WIDTH as f32, CHUNK_HEIGHT as f32) * tile_size;
                let chunk_pixel_size = chunk_pixel_size * scale.truncate();

                let chunks_changed_at = &mut extracted_tilemaps.chunks_changed_at;

                let chunk_iter = tilemap.chunks.iter();

                // Exclude chunks that are not visible
                let chunks: Vec<_> = chunk_iter
                    .filter_map(|(_, chunk)| {
                        let chunk_translation =
                            (chunk.origin.truncate().as_vec2() * tile_size).extend(chunk.origin.z as f32);
                        let chunk_translation = transform.mul(chunk_translation);

                        let chunk_rect = Rect {
                            anchor: Anchor::BottomLeft,
                            position: chunk_translation.truncate(),
                            size: chunk_pixel_size,
                        };

                        if camera_rects.iter().all(|cr| !cr.is_intersecting(&chunk_rect)) {
                            // Chunk is outside the camera, skip it.
                            return None;
                        }

                        Some(chunk)
                    })
                    .collect();

                let visible_chunks: Vec<IVec3> = chunks.iter().map(|c| c.origin).collect();

                #[cfg(target_arch = "wasm32")]
                let chunk_iter = chunks.iter();
                #[cfg(all(not(target_arch = "wasm32"), not(feature = "rayon")))]
                let chunk_iter = chunks.iter();
                #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
                let chunk_iter = chunks.par_iter();

                // Extract chunks
                let chunks: Vec<ExtractedChunk> = chunk_iter
                    .filter_map(|chunk| {
                        // If chunk has not changed since last extraction, skip it.
                        if let Some(chunk_changed_at) = chunks_changed_at.get(&(entity, chunk.origin)) {
                            if chunk.last_change_at <= *chunk_changed_at {
                                return None;
                            }
                        }

                        #[cfg(target_arch = "wasm32")]
                        let tile_iter = chunk.tiles.iter();
                        #[cfg(all(not(target_arch = "wasm32"), not(feature = "rayon")))]
                        let tile_iter = chunk.tiles.iter();
                        #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
                        let tile_iter = chunk.tiles.par_iter();

                        let tiles: Vec<ExtractedTile> = tile_iter
                            .enumerate()
                            .filter_map(|(i, tile)| {
                                if let Some(tile) = tile {
                                    let rect = texture_atlas.textures[tile.sprite_index as usize];

                                    Some(ExtractedTile {
                                        pos: chunk.origin.truncate() + row_major_pos(i),
                                        rect,
                                        color: tile.color,
                                        flags: tile.flags,
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();

                        Some(ExtractedChunk {
                            origin: chunk.origin,
                            tiles,
                            last_change_at: chunk.last_change_at,
                        })
                    })
                    .collect();

                // Update chunk change timestamps
                for ec in chunks.iter() {
                    chunks_changed_at.insert((entity, ec.origin), ec.last_change_at);
                }

                extracted_tilemaps.tilemaps.push(ExtractedTilemap {
                    entity,
                    transform: *transform,
                    image_handle_id: texture_atlas.texture.id(),
                    tile_size,
                    atlas_size: texture_atlas.size,
                    chunks,
                    visible_chunks,
                });
            }
        }
    }
}
