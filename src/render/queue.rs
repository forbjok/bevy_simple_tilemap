use std::cmp::Ordering;

use bevy::asset::{AssetEvent, Handle};
use bevy::core_pipeline::core_2d::Transparent2d;
use bevy::ecs::prelude::*;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::render::{
    render_asset::RenderAssets,
    render_phase::{DrawFunctions, RenderPhase},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    view::ViewUniforms,
};

use bevy::utils::FloatOrd;

#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::TileFlags;

use super::draw::DrawTilemap;
use super::pipeline::{TilemapPipeline, TilemapPipelineKey};
use super::*;

const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    Vec2::from_array([-0.5, -0.5]),
    Vec2::from_array([0.5, -0.5]),
    Vec2::from_array([0.5, 0.5]),
    Vec2::from_array([-0.5, 0.5]),
];

const QUAD_UVS: [Vec2; 4] = [
    Vec2::from_array([0., 1.]),
    Vec2::from_array([1., 1.]),
    Vec2::from_array([1., 0.]),
    Vec2::from_array([0., 0.]),
];

#[allow(clippy::too_many_arguments)]
pub fn queue_tilemaps(
    mut commands: Commands,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut tilemap_meta: ResMut<TilemapMeta>,
    view_uniforms: Res<ViewUniforms>,
    tilemap_pipeline: Res<TilemapPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TilemapPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    msaa: Res<Msaa>,
    mut extracted_tilemaps: ResMut<ExtractedTilemaps>,
    mut views: Query<&mut RenderPhase<Transparent2d>>,
    events: Res<TilemapAssetEvents>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Created { .. } => None,
            AssetEvent::Modified { handle } => image_bind_groups.values.remove(handle),
            AssetEvent::Removed { handle } => image_bind_groups.values.remove(handle),
        };
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let tilemap_meta = &mut tilemap_meta;

        tilemap_meta.view_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_binding,
            }],
            label: Some("tilemap_view_bind_group"),
            layout: &tilemap_pipeline.view_layout,
        }));

        let draw_tilemap_function = draw_functions.read().get_id::<DrawTilemap>().unwrap();
        let key = TilemapPipelineKey::from_msaa_samples(msaa.samples());
        let pipeline = pipelines.specialize(&pipeline_cache, &tilemap_pipeline, key);

        for mut transparent_phase in views.iter_mut() {
            let tilemaps = &mut extracted_tilemaps.tilemaps;
            let image_bind_groups = &mut *image_bind_groups;

            transparent_phase.items.reserve(tilemaps.len());

            let mut visible_chunks: Vec<(Entity, IVec3)> = Vec::new();
            let mut tilemap_transforms: HashMap<Entity, GlobalTransform> = HashMap::default();
            let mut tilemap_image_handle_ids: HashMap<Entity, HandleId> = HashMap::default();

            for tilemap in tilemaps.iter_mut() {
                let image_size;

                // Set-up a new possible batch
                if let Some(gpu_image) = gpu_images.get(&Handle::weak(tilemap.image_handle_id)) {
                    image_size = gpu_image.size;

                    image_bind_groups
                        .values
                        .entry(Handle::weak(tilemap.image_handle_id))
                        .or_insert_with(|| {
                            render_device.create_bind_group(&BindGroupDescriptor {
                                entries: &[
                                    BindGroupEntry {
                                        binding: 0,
                                        resource: BindingResource::TextureView(&gpu_image.texture_view),
                                    },
                                    BindGroupEntry {
                                        binding: 1,
                                        resource: BindingResource::Sampler(&gpu_image.sampler),
                                    },
                                ],
                                label: Some("tilemap_material_bind_group"),
                                layout: &tilemap_pipeline.material_layout,
                            })
                        });
                } else {
                    // Skip this item if the texture is not ready
                    continue;
                }

                // Yank each chunk's GPU metadata (if one exists) out of the HashMap
                // so that we can pass it into the parallel iterator later.
                // Maybe there is a cleaner way of doing this, but I can't think of one
                // so this will have to do for now.
                let chonks: Vec<(ExtractedChunk, Option<(ChunkKey, ChunkMeta)>)> = tilemap
                    .chunks
                    .drain(..)
                    .map(|c| {
                        let entry = tilemap_meta.chunks.remove_entry(&(tilemap.entity, c.origin));

                        (c, entry)
                    })
                    .collect();

                #[cfg(target_arch = "wasm32")]
                let chonk_iter = chonks.into_iter();
                #[cfg(all(not(target_arch = "wasm32"), not(feature = "rayon")))]
                let chonk_iter = chonks.into_iter();
                #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
                let chonk_iter = chonks.into_par_iter();

                // Process extracted chunks in parallel, updating their metadata.
                let results: Vec<(ChunkKey, ChunkMeta)> = chonk_iter
                    .map(|(chunk, chunk_meta)| {
                        let (key, mut chunk_meta) = if let Some((key, chunk_meta)) = chunk_meta {
                            (key, chunk_meta)
                        } else {
                            ((tilemap.entity, chunk.origin), ChunkMeta::default())
                        };

                        chunk_meta.tile_size = tilemap.tile_size;
                        chunk_meta.texture_size = image_size;
                        chunk_meta.vertices.clear();

                        let z = chunk.origin.z as f32;

                        for tile in chunk.tiles.iter() {
                            // Calculate vertex data for this item

                            let mut uvs = QUAD_UVS;

                            if tile.flags.contains(TileFlags::FLIP_X) {
                                uvs = [uvs[1], uvs[0], uvs[3], uvs[2]];
                            }

                            if tile.flags.contains(TileFlags::FLIP_Y) {
                                uvs = [uvs[3], uvs[2], uvs[1], uvs[0]];
                            }

                            let tile_uvs = uvs;

                            // If a rect is specified, adjust UVs and the size of the quad
                            let rect = tile.rect;
                            let quad_size = rect.size();
                            for uv in &mut uvs {
                                *uv = (rect.min + *uv * quad_size) / image_size;
                            }

                            let tile_pos = tile.pos.as_vec2() * quad_size;

                            // Apply size and global transform
                            let positions = QUAD_VERTEX_POSITIONS
                                .map(|quad_pos| (tile_pos + (quad_pos * quad_size)).extend(z).into());

                            // Store the vertex data and add the item to the render phase
                            let color = tile.color.as_linear_rgba_f32();
                            // encode color as a single u32 to save space
                            let color = (color[0] * 255.0) as u32
                                | ((color[1] * 255.0) as u32) << 8
                                | ((color[2] * 255.0) as u32) << 16
                                | ((color[3] * 255.0) as u32) << 24;

                            for i in QUAD_INDICES.iter() {
                                chunk_meta.vertices.push(TilemapVertex {
                                    position: positions[*i],
                                    uv: uvs[*i].into(),
                                    tile_uv: tile_uvs[*i].into(),
                                    color,
                                });
                            }
                        }

                        (key, chunk_meta)
                    })
                    .collect();

                // (Re-)Insert chunk metadata into the HashMap
                for (key, chunk_meta) in results {
                    tilemap_meta.chunks.insert(key, chunk_meta);
                }

                visible_chunks.extend(tilemap.visible_chunks.drain(..).map(|pos| (tilemap.entity, pos)));
                tilemap_transforms.insert(tilemap.entity, tilemap.transform);
                tilemap_image_handle_ids.insert(tilemap.entity, tilemap.image_handle_id);
            }

            let mut sorted_chunks: Vec<_> = tilemap_meta
                .chunks
                .iter_mut()
                .filter(|(key, _)| {
                    // If chunk is not visible, there is no need to draw it.
                    visible_chunks.contains(key)
                })
                .map(|(key, chunk_meta)| {
                    let (entity, _) = key;
                    let tilemap_transform = tilemap_transforms.get(entity).unwrap();

                    (key, tilemap_transform, chunk_meta)
                })
                .collect();

            sorted_chunks.sort_unstable_by(|((_, a), att, _), ((_, b), btt, _)| {
                let att_translation = att.translation();
                let btt_translation = btt.translation();

                match att_translation.z.partial_cmp(&btt_translation.z) {
                    Some(Ordering::Equal) | None => a.z.cmp(&b.z),
                    Some(other) => other,
                }
            });

            // Render all chunks.
            for (key, tilemap_transform, chunk_meta) in sorted_chunks.into_iter() {
                let (tilemap_entity, _) = key;

                let batch = TilemapBatch {
                    chunk_key: *key,
                    image_handle_id: *tilemap_image_handle_ids.get(tilemap_entity).unwrap(),
                };

                let batch_entity = commands.spawn((batch,)).id();

                chunk_meta.tilemap_gpu_data.clear();
                chunk_meta.tilemap_gpu_data.push(TilemapGpuData {
                    transform: tilemap_transform.compute_matrix(),
                    tile_size: chunk_meta.tile_size,
                    texture_size: chunk_meta.texture_size,
                });

                chunk_meta.tilemap_gpu_data.write_buffer(&render_device, &render_queue);
                chunk_meta.vertices.write_buffer(&render_device, &render_queue);

                chunk_meta.tilemap_gpu_data_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: chunk_meta.tilemap_gpu_data.binding().unwrap(),
                    }],
                    label: Some("tilemap_gpu_data_bind_group"),
                    layout: &tilemap_pipeline.tilemap_gpu_data_layout,
                }));

                let translation = tilemap_transform.translation();

                // These items will be sorted by depth with other phase items
                let sort_key = FloatOrd(translation.z);

                let vertex_count = chunk_meta.vertices.len() as u32;

                transparent_phase.add(Transparent2d {
                    draw_function: draw_tilemap_function,
                    pipeline,
                    entity: batch_entity,
                    sort_key,
                    batch_range: Some(0..vertex_count),
                });
            }
        }
    }
}
