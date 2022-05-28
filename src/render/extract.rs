use bevy::asset::{AssetEvent, Assets, Handle};
use bevy::ecs::prelude::*;
use bevy::math::uvec2;
use bevy::prelude::*;
use bevy::render::camera::{ActiveCamera, Camera2d};
use bevy::render::{render_resource::TextureUsages, texture::Image, view::ComputedVisibility, RenderWorld};
use bevy::transform::components::GlobalTransform;

#[cfg(not(target_arch = "wasm32"))]
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::tilemap::{row_major_pos, CHUNK_HEIGHT, CHUNK_WIDTH};
use crate::TileMap;

use super::*;

pub fn extract_tilemap_events(mut render_world: ResMut<RenderWorld>, mut image_events: EventReader<AssetEvent<Image>>) {
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
}

pub fn extract_tilemaps(
    mut render_world: ResMut<RenderWorld>,
    images: Res<Assets<Image>>,
    tilemap_query: Query<(Entity, &ComputedVisibility, &TileMap, &GlobalTransform, &Handle<Image>)>,
    windows: Res<Windows>,
    active_camera: Res<ActiveCamera<Camera2d>>,
    camera_transform_query: Query<&GlobalTransform, With<Camera2d>>,
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

    let window = windows.get_primary().unwrap();
    let window_size = Vec2::new(window.width(), window.height());

    let camera_rects = {
        let mut camera_rects: Vec<Rect> = Vec::with_capacity(3);

        if let Some(camera_entity) = active_camera.get() {
            if let Ok(camera_transform) = camera_transform_query.get(camera_entity) {
                let camera_size = window_size * camera_transform.scale.truncate();

                let camera_rect = Rect {
                    anchor: Anchor::Center,
                    position: camera_transform.translation.truncate(),
                    size: camera_size,
                };

                camera_rects.push(camera_rect);
            }
        }

        camera_rects
    };

    let mut extracted_tilemaps = render_world.get_resource_mut::<ExtractedTilemaps>().unwrap();
    extracted_tilemaps.tilemaps.clear();
    for (entity, computed_visibility, tilemap, transform, texture) in tilemap_query.iter() {
        if !computed_visibility.is_visible {
            continue;
        }

        if let Some(image) = images.get(texture) {
            if !image.texture_descriptor.usage.contains(TextureUsages::COPY_SRC) {
                continue;
            }

            let texture_size = uvec2(
                image.texture_descriptor.size.width,
                image.texture_descriptor.size.height,
            );

            let tile_size = tilemap.tile_size;

            let chunk_pixel_size = uvec2(CHUNK_WIDTH, CHUNK_HEIGHT) * tile_size;
            let chunk_pixel_size = chunk_pixel_size.as_vec2() * transform.scale.truncate();

            let chunks_changed_at = &mut extracted_tilemaps.chunks_changed_at;

            let chunk_iter = tilemap.chunks.iter();

            // Exclude chunks that are not visible
            let chunks: Vec<_> = chunk_iter
                .filter_map(|(_, chunk)| {
                    let chunk_translation =
                        (chunk.origin.truncate().as_vec2() * tile_size.as_vec2()).extend(chunk.origin.z as f32);
                    let chunk_translation = transform.mul_vec3(chunk_translation);

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
            #[cfg(not(target_arch = "wasm32"))]
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
                    #[cfg(not(target_arch = "wasm32"))]
                    let tile_iter = chunk.tiles.par_iter();

                    let tiles: Vec<ExtractedTile> = tile_iter
                        .enumerate()
                        .filter_map(|(i, tile)| {
                            tile.as_ref().map(|tile| ExtractedTile {
                                pos: chunk.origin.truncate() + row_major_pos(i),
                                index: tile.sprite_index,
                                color: tile.color,
                                flags: tile.flags,
                            })
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
                tile_size,
                texture_size,
                padding: tilemap.padding,
                transform: *transform,
                texture: texture.clone(),
                chunks,
                visible_chunks,
            });
        }
    }
}
