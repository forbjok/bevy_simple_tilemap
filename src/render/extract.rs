use bevy::asset::{AssetEvent, Assets, Handle};
use bevy::ecs::prelude::*;
use bevy::prelude::*;
use bevy::render::camera::ActiveCameras;
use bevy::render::{texture::Image, view::ComputedVisibility, RenderWorld};
use bevy::sprite::TextureAtlas;
use bevy::transform::components::GlobalTransform;
use bevy::utils::Instant;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator, IndexedParallelIterator};

use crate::tilemap::{row_major_pos, CHUNK_WIDTH, CHUNK_HEIGHT};
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
    windows: Res<Windows>,
    active_cameras: Res<ActiveCameras>,
    camera_transform_query: Query<&GlobalTransform, With<Camera>>,
) {
    //let timer = Instant::now();

    #[derive(Debug)]
    enum Anchor {
        BottomLeft,
        Center,
    }

    #[derive(Debug)]
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

        for active_camera_entity in active_cameras.iter().filter_map(|a| a.entity) {
            if let Ok(camera_transform) = camera_transform_query.get(active_camera_entity) {
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
    for (computed_visibility, tilemap, transform, texture_atlas_handle) in tilemap_query.iter() {
        if !computed_visibility.is_visible {
            continue;
        }

        if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
            if images.contains(&texture_atlas.texture) {
                // Determine tile size in pixels from first sprite in TextureAtlas.
                // It is assumed and mandated that all sprites in the sprite sheet are the same size.
                let tile0_tex = texture_atlas.textures.get(0).unwrap();
                let tile_size = Vec2::new(tile0_tex.width(), tile0_tex.height());

                let chunk_pixel_size = Vec2::new(CHUNK_WIDTH as f32, CHUNK_HEIGHT as f32) * tile_size;
                let chunk_pixel_size = chunk_pixel_size * transform.scale.truncate();

                let chunks: Vec<ExtractedChunk> = tilemap.chunks.par_iter().filter_map(|(_pos, chunk)| {
                    let chunk_translation = (chunk.origin.truncate().as_vec2() * tile_size).extend(chunk.origin.z as f32);
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

                    let tiles: Vec<ExtractedTile> = chunk.tiles.par_iter().enumerate().filter_map(|(i, tile)| {
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
                    }).collect();

                    Some(ExtractedChunk {
                        origin: chunk.origin,
                        tiles,
                    })
                }).collect();

                extracted_tilemaps.tilemaps.push(ExtractedTilemap {
                    transform: *transform,
                    image_handle_id: texture_atlas.texture.id,
                    atlas_size: texture_atlas.size,
                    chunks,
                });
            }
        }
    }
    //info!("EXT TILEMAP {:?}", timer.elapsed());
}
