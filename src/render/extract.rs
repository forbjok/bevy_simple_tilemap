use std::ops::Mul;

use bevy::asset::{AssetEvent, Assets};
use bevy::ecs::prelude::*;
use bevy::math::uvec2;
use bevy::prelude::*;
use bevy::render::texture::Image;
use bevy::render::Extract;
use bevy::sprite::TextureAtlas;
use bevy::transform::components::GlobalTransform;

#[cfg(not(target_arch = "wasm32"))]
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

    for event in image_events.read() {
        images.push(*event);
    }
}

#[allow(clippy::type_complexity)]
pub fn extract_tilemaps(
    mut extracted_tilemaps: ResMut<ExtractedTilemaps>,
    images: Extract<Res<Assets<Image>>>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    tilemap_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &TileMap,
            &GlobalTransform,
            &Handle<Image>,
            &TextureAtlas,
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

    for (entity, view_visibility, tilemap, transform, texture, atlas) in tilemap_query.iter() {
        if !view_visibility.get() {
            continue;
        }

        if let Some(texture_atlas) = texture_atlases.get(&atlas.layout) {
            if images.contains(texture) {
                let (scale, _, _) = transform.to_scale_rotation_translation();

                // Determine tile size in pixels from first sprite in TextureAtlas.
                // It is assumed and mandated that all sprites in the sprite sheet are the same size.
                let tile0_tex = texture_atlas.textures.first().unwrap();
                let tile_size = uvec2(tile0_tex.width(), tile0_tex.height());

                let chunk_pixel_size = uvec2(CHUNK_WIDTH, CHUNK_HEIGHT) * tile_size;
                let chunk_pixel_size = chunk_pixel_size * scale.truncate().as_uvec2();

                let chunk_iter = tilemap.chunks.iter();

                // Exclude chunks that are not visible
                let chunks: Vec<_> = chunk_iter
                    .filter_map(|(_, chunk)| {
                        let chunk_translation =
                            (chunk.origin.truncate().as_vec2() * tile_size.as_vec2()).extend(chunk.origin.z as f32);
                        let chunk_translation = transform.mul(chunk_translation);

                        let chunk_rect = Rect {
                            anchor: Anchor::BottomLeft,
                            position: chunk_translation.truncate(),
                            size: chunk_pixel_size.as_vec2(),
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
                #[cfg(not(target_arch = "wasm32"))]
                let chunk_iter = chunks.par_iter();

                // Extract chunks
                let chunks: Vec<ExtractedChunk> = chunk_iter
                    .filter_map(|chunk| {
                        #[cfg(target_arch = "wasm32")]
                        let tile_iter = chunk.tiles.iter();
                        #[cfg(not(target_arch = "wasm32"))]
                        let tile_iter = chunk.tiles.par_iter();

                        let tiles: Vec<ExtractedTile> = tile_iter
                            .enumerate()
                            .filter_map(|(i, tile)| {
                                if let Some(tile) = tile {
                                    let rect = texture_atlas.textures[tile.sprite_index as usize];

                                    Some(ExtractedTile {
                                        pos: chunk.origin.truncate() + row_major_pos(i),
                                        rect,
                                        color: tile.color.into(),
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
                        })
                    })
                    .collect();

                extracted_tilemaps.tilemaps.push(ExtractedTilemap {
                    entity,
                    transform: *transform,
                    image_handle_id: texture.id(),
                    tile_size,
                    chunks,
                    visible_chunks,
                });
            }
        }
    }
}
