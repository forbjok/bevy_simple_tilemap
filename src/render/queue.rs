use std::cmp::Ordering;

use bevy::asset::{AssetEvent, Handle};
use bevy::core::FloatOrd;
use bevy::core_pipeline::Transparent2d;
use bevy::ecs::prelude::*;
use bevy::math::{const_vec2, Vec2};
use bevy::prelude::{Msaa, info};
use bevy::render::{
    render_asset::RenderAssets,
    render_phase::{DrawFunctions, RenderPhase},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    view::ViewUniforms,
};
use bevy::utils::Instant;

use crate::TileFlags;

use super::draw::DrawTilemap;
use super::pipeline::{TilemapPipeline, TilemapPipelineKey};
use super::*;

const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    const_vec2!([-0.5, -0.5]),
    const_vec2!([0.5, -0.5]),
    const_vec2!([0.5, 0.5]),
    const_vec2!([-0.5, 0.5]),
];

const QUAD_UVS: [Vec2; 4] = [
    const_vec2!([0., 1.]),
    const_vec2!([1., 1.]),
    const_vec2!([1., 0.]),
    const_vec2!([0., 0.]),
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
    mut pipelines: ResMut<SpecializedPipelines<TilemapPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    msaa: Res<Msaa>,
    mut extracted_tilemaps: ResMut<ExtractedTilemaps>,
    mut views: Query<&mut RenderPhase<Transparent2d>>,
    events: Res<TilemapAssetEvents>,
) {
    let timer = Instant::now();

    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Created { .. } => None,
            AssetEvent::Modified { handle } => image_bind_groups.values.remove(handle),
            AssetEvent::Removed { handle } => image_bind_groups.values.remove(handle),
        };
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let sprite_meta = &mut tilemap_meta;

        // Clear the vertex buffers
        sprite_meta.vertices.clear();

        sprite_meta.view_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_binding,
            }],
            label: Some("tilemap_view_bind_group"),
            layout: &tilemap_pipeline.view_layout,
        }));

        let draw_tilemap_function = draw_functions.read().get_id::<DrawTilemap>().unwrap();
        let key = TilemapPipelineKey::from_msaa_samples(msaa.samples);
        let pipeline = pipelines.specialize(&mut pipeline_cache, &tilemap_pipeline, key);

        // Vertex buffer indices
        let mut index = 0;

        // FIXME: VisibleEntities is ignored
        for mut transparent_phase in views.iter_mut() {
            let tilemaps = &mut extracted_tilemaps.tilemaps;
            let image_bind_groups = &mut *image_bind_groups;

            transparent_phase.items.reserve(tilemaps.len());

            // Sort sprites by z for correct transparency and then by handle to improve batching
            tilemaps.sort_unstable_by(
                |a, b| match a.transform.translation.z.partial_cmp(&b.transform.translation.z) {
                    Some(Ordering::Equal) | None => a.image_handle_id.cmp(&b.image_handle_id),
                    Some(other) => other,
                },
            );

            for tilemap in tilemaps.iter() {
                let batch = TilemapBatch {
                    image_handle_id: tilemap.image_handle_id,
                };

                let image_size;
                let batch_entity;

                let item_start = index;

                // Set-up a new possible batch
                if let Some(gpu_image) = gpu_images.get(&Handle::weak(batch.image_handle_id)) {
                    image_size = Vec2::new(gpu_image.size.width, gpu_image.size.height);
                    batch_entity = commands.spawn_bundle((batch,)).id();

                    image_bind_groups
                        .values
                        .entry(Handle::weak(batch.image_handle_id))
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
                                label: Some("sprite_material_bind_group"),
                                layout: &tilemap_pipeline.material_layout,
                            })
                        });
                } else {
                    // Skip this item if the texture is not ready
                    continue;
                }

                for chunk in tilemap.chunks.iter() {
                    for tile in chunk.tiles.iter() {
                        // Calculate vertex data for this item

                        let mut uvs = QUAD_UVS;

                        if tile.flags.contains(TileFlags::FLIP_X) {
                            uvs = [uvs[1], uvs[0], uvs[3], uvs[2]];
                        }

                        if tile.flags.contains(TileFlags::FLIP_Y) {
                            uvs = [uvs[3], uvs[2], uvs[1], uvs[0]];
                        }

                        // By default, the size of the quad is the size of the texture
                        //let mut quad_size = image_size;

                        // If a rect is specified, adjust UVs and the size of the quad
                        let rect = tile.rect;
                        let rect_size = rect.size();
                        for uv in &mut uvs {
                            *uv = (rect.min + *uv * rect_size) / image_size;
                        }

                        let quad_size = rect_size;

                        // Override the size if a custom one is specified
                        //if let Some(custom_size) = extracted_sprite.custom_size {
                        //    quad_size = custom_size;
                        //}

                        let tile_pos = tile.pos.as_vec2() * quad_size; // TODO: Make work

                        // Apply size and global transform
                        let positions = QUAD_VERTEX_POSITIONS.map(|quad_pos| {
                            tilemap
                                .transform
                                .mul_vec3((tile_pos + (quad_pos * quad_size)).extend(0.))
                                .into()
                        });

                        // Store the vertex data and add the item to the render phase
                        let color = tile.color.as_linear_rgba_f32();
                        // encode color as a single u32 to save space
                        let color = (color[0] * 255.0) as u32
                            | ((color[1] * 255.0) as u32) << 8
                            | ((color[2] * 255.0) as u32) << 16
                            | ((color[3] * 255.0) as u32) << 24;

                        for i in QUAD_INDICES.iter() {
                            sprite_meta.vertices.push(TilemapVertex {
                                position: positions[*i],
                                uv: uvs[*i].into(),
                                color,
                            });
                        }

                        index += QUAD_INDICES.len() as u32;
                    }
                }

                let item_end = index;

                // These items will be sorted by depth with other phase items
                let sort_key = FloatOrd(tilemap.transform.translation.z);

                transparent_phase.add(Transparent2d {
                    draw_function: draw_tilemap_function,
                    pipeline,
                    entity: batch_entity,
                    sort_key,
                    batch_range: Some(item_start..item_end),
                });
            }
        }
        sprite_meta.vertices.write_buffer(&render_device, &render_queue);
    }

    info!("QUEUE {:?}", timer.elapsed());
}
