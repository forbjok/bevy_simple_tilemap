use std::{cmp::Ordering, ops::Range};

use bevy::prelude::HandleUntyped;
use bevy::reflect::TypeUuid;
use bevy::sprite::{Rect, TextureAtlas};
use bevy::asset::{AssetEvent, Assets, Handle};
use bevy::core::FloatOrd;
use bevy::core_pipeline::Transparent2d;
use bevy::ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemState},
};
use bevy::math::{const_vec3, Mat4, Vec2, Vec3, Vec4Swizzles, IVec2};
use bevy::render::{
    color::Color,
    render_asset::RenderAssets,
    render_phase::{Draw, DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, Image},
    view::{ComputedVisibility, ViewUniform, ViewUniformOffset, ViewUniforms},
    RenderWorld,
};
use bevy::transform::components::GlobalTransform;
use bevy::utils::HashMap;
use bytemuck::{Pod, Zeroable};
use crevice::std140::AsStd140;

use crate::{TileMap, TileFlags};
use crate::tilemap::row_major_pos;

pub const TILEMAP_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 9765236402292098257);

pub struct TilemapPipeline {
    view_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
}

impl FromWorld for TilemapPipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: BufferSize::new(ViewUniform::std140_size_static() as u64),
                },
                count: None,
            }],
            label: Some("tilemap_view_layout"),
        });

        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("tilemap_material_layout"),
        });

        TilemapPipeline {
            view_layout,
            material_layout,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct TilemapPipelineKey;

impl SpecializedPipeline for TilemapPipeline {
    type Key = TilemapPipelineKey;

    fn specialize(&self, _key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: 24,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 12,
                    shader_location: 1,
                },
                // Color
                VertexAttribute {
                    format: VertexFormat::Uint32,
                    offset: 20,
                    shader_location: 2,
                },
            ],
        };

        let shader_defs = Vec::new();

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: TILEMAP_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: TILEMAP_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            layout: Some(vec![self.view_layout.clone(), self.material_layout.clone()]),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("tilemap_pipeline".into()),
        }
    }
}

pub struct ExtractedTile {
    pub pos: IVec2,
    pub rect: Rect,
    pub color: Color,
    pub flags: TileFlags,
}

pub struct ExtractedTilemap {
    pub transform: Mat4,
    pub handle: Handle<Image>,
    pub atlas_size: Vec2,
    pub tiles: Vec<ExtractedTile>,
}

#[derive(Default)]
pub struct ExtractedTilemaps {
    pub tilemaps: Vec<ExtractedTilemap>,
}

#[derive(Default)]
pub struct TilemapAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

pub fn extract_tilemap_events(
    mut render_world: ResMut<RenderWorld>,
    mut image_events: EventReader<AssetEvent<Image>>,
) {
    let mut events = render_world
        .get_resource_mut::<TilemapAssetEvents>()
        .unwrap();
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
    texture_atlases: Res<Assets<TextureAtlas>>,
    tilemap_query: Query<(
        &ComputedVisibility,
        &TileMap,
        &GlobalTransform,
        &Handle<TextureAtlas>,
    )>,
) {
    let mut extracted_tilemaps = render_world.get_resource_mut::<ExtractedTilemaps>().unwrap();
    extracted_tilemaps.tilemaps.clear();
    for (computed_visibility, tilemap, transform, texture_atlas_handle) in tilemap_query.iter() {
        if !computed_visibility.is_visible {
            continue;
        }

        if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
            if images.contains(&texture_atlas.texture) {
                let mut tiles: Vec<ExtractedTile> = Vec::new();

                for (_pos, chunk) in tilemap.chunks.iter() {
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
                }

                extracted_tilemaps.tilemaps.push(ExtractedTilemap {
                    transform: transform.compute_matrix(),
                    handle: texture_atlas.texture.clone(),
                    atlas_size: texture_atlas.size,
                    tiles,
                });
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct TilemapVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: u32,
}

/// Probably a cache of GPU data to be used in shaders?
pub struct TilemapMeta {
    vertices: BufferVec<TilemapVertex>,
    view_bind_group: Option<BindGroup>,
}

impl Default for TilemapMeta {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
            view_bind_group: None,
        }
    }
}

const QUAD_VERTEX_POSITIONS: &[Vec3] = &[
    const_vec3!([-0.5, -0.5, 0.0]),
    const_vec3!([0.5, 0.5, 0.0]),
    const_vec3!([-0.5, 0.5, 0.0]),
    const_vec3!([-0.5, -0.5, 0.0]),
    const_vec3!([0.5, -0.5, 0.0]),
    const_vec3!([0.5, 0.5, 0.0]),
];

#[derive(Component)]
pub struct TilemapBatch {
    range: Range<u32>,
    handle: Handle<Image>,
    z: f32,
}

pub fn prepare_tilemaps(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut tilemap_meta: ResMut<TilemapMeta>,
    mut extracted_tilemaps: ResMut<ExtractedTilemaps>,
) {
    tilemap_meta.vertices.clear();

    // sort first by z and then by handle. this ensures that, when possible, batches span multiple z layers
    // batches won't span z-layers if there is another batch between them
    extracted_tilemaps.tilemaps.sort_by(|a, b| {
        match FloatOrd(a.transform.w_axis[2]).cmp(&FloatOrd(b.transform.w_axis[2])) {
            Ordering::Equal => a.handle.cmp(&b.handle),
            other => other,
        }
    });

    let start = 0;
    let mut end = 0;
    let mut current_batch_handle: Option<Handle<Image>> = None;
    let mut last_z = 0.0;

    for extracted_tilemap in extracted_tilemaps.tilemaps.iter() {
        if let Some(current_batch_handle) = &current_batch_handle {
            if *current_batch_handle != extracted_tilemap.handle
            {
                commands.spawn_bundle((TilemapBatch {
                    range: start..end,
                    handle: current_batch_handle.clone_weak(),
                    z: last_z,
                },));
            }
        }
        current_batch_handle = Some(extracted_tilemap.handle.clone_weak());

        for tile in extracted_tilemap.tiles.iter() {
            let tilemap_rect = tile.rect;

            // Specify the corners of the sprite
            let mut bottom_left = Vec2::new(tilemap_rect.min.x, tilemap_rect.max.y);
            let mut top_left = tilemap_rect.min;
            let mut top_right = Vec2::new(tilemap_rect.max.x, tilemap_rect.min.y);
            let mut bottom_right = tilemap_rect.max;

            if tile.flags.contains(TileFlags::FLIP_X) {
                bottom_left.x = tilemap_rect.max.x;
                top_left.x = tilemap_rect.max.x;
                bottom_right.x = tilemap_rect.min.x;
                top_right.x = tilemap_rect.min.x;
            }

            if tile.flags.contains(TileFlags::FLIP_Y) {
                bottom_left.y = tilemap_rect.min.y;
                bottom_right.y = tilemap_rect.min.y;
                top_left.y = tilemap_rect.max.y;
                top_right.y = tilemap_rect.max.y;
            }

            let atlas_extent = extracted_tilemap.atlas_size;
            bottom_left /= atlas_extent;
            bottom_right /= atlas_extent;
            top_left /= atlas_extent;
            top_right /= atlas_extent;

            let uvs: [[f32; 2]; 6] = [
                bottom_left.into(),
                top_right.into(),
                top_left.into(),
                bottom_left.into(),
                bottom_right.into(),
                top_right.into(),
            ];

            let rect_size = tile.rect.size().extend(1.0);
            let color = tile.color.as_linear_rgba_f32();
            let tile_pos = tile.pos.as_vec2().extend(1.0) * rect_size;

            // encode color as a single u32 to save space
            let color = (color[0] * 255.0) as u32
                | ((color[1] * 255.0) as u32) << 8
                | ((color[2] * 255.0) as u32) << 16
                | ((color[3] * 255.0) as u32) << 24;

            for (index, vertex_position) in QUAD_VERTEX_POSITIONS.iter().enumerate() {
                let mut final_position = tile_pos + (*vertex_position * rect_size);

                final_position = (extracted_tilemap.transform * final_position.extend(1.0)).xyz();

                tilemap_meta.vertices.push(TilemapVertex {
                    position: final_position.into(),
                    uv: uvs[index],
                    color,
                });
            }

            last_z = extracted_tilemap.transform.w_axis[2];
            end += QUAD_VERTEX_POSITIONS.len() as u32;
        }

    }

    // if start != end, there is one last batch to process
    if start != end {
        if let Some(current_batch_handle) = current_batch_handle {
            commands.spawn_bundle((TilemapBatch {
                range: start..end,
                handle: current_batch_handle,
                z: last_z,
            },));
        }
    }

    tilemap_meta
        .vertices
        .write_buffer(&render_device, &render_queue);
}

#[derive(Default)]
pub struct ImageBindGroups {
    values: HashMap<Handle<Image>, BindGroup>,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_tilemaps(
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    render_device: Res<RenderDevice>,
    mut tilemap_meta: ResMut<TilemapMeta>,
    view_uniforms: Res<ViewUniforms>,
    tilemap_pipeline: Res<TilemapPipeline>,
    mut pipelines: ResMut<SpecializedPipelines<TilemapPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    mut tilemap_batches: Query<(Entity, &TilemapBatch)>,
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
        tilemap_meta.view_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_binding,
            }],
            label: Some("tilemap_view_bind_group"),
            layout: &tilemap_pipeline.view_layout,
        }));
        let draw_tilemap_function = draw_functions.read().get_id::<DrawTilemap>().unwrap();
        let pipeline = pipelines.specialize(
            &mut pipeline_cache,
            &tilemap_pipeline,
            TilemapPipelineKey,
        );

        for mut transparent_phase in views.iter_mut() {
            for (entity, batch) in tilemap_batches.iter_mut() {
                image_bind_groups
                    .values
                    .entry(batch.handle.clone_weak())
                    .or_insert_with(|| {
                        let gpu_image = gpu_images.get(&batch.handle).unwrap();
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
                transparent_phase.add(Transparent2d {
                    draw_function: draw_tilemap_function,
                    pipeline,
                    entity,
                    sort_key: FloatOrd(batch.z),
                });
            }
        }
    }
}

pub struct DrawTilemap {
    params: SystemState<(
        SRes<TilemapMeta>,
        SRes<ImageBindGroups>,
        SRes<RenderPipelineCache>,
        SQuery<Read<ViewUniformOffset>>,
        SQuery<Read<TilemapBatch>>,
    )>,
}

impl DrawTilemap {
    pub fn new(world: &mut World) -> Self {
        Self {
            params: SystemState::new(world),
        }
    }
}

impl Draw<Transparent2d> for DrawTilemap {
    fn draw<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        item: &Transparent2d,
    ) {
        let (tilemap_meta, image_bind_groups, pipelines, views, sprites) = self.params.get(world);
        let view_uniform = views.get(view).unwrap();
        let tilemap_meta = tilemap_meta.into_inner();
        let image_bind_groups = image_bind_groups.into_inner();
        let tilemap_batch = sprites.get(item.entity).unwrap();
        if let Some(pipeline) = pipelines.into_inner().get(item.pipeline) {
            pass.set_render_pipeline(pipeline);
            pass.set_vertex_buffer(0, tilemap_meta.vertices.buffer().unwrap().slice(..));
            pass.set_bind_group(
                0,
                tilemap_meta.view_bind_group.as_ref().unwrap(),
                &[view_uniform.offset],
            );
            pass.set_bind_group(
                1,
                image_bind_groups.values.get(&tilemap_batch.handle).unwrap(),
                &[],
            );

            pass.draw(tilemap_batch.range.clone(), 0..1);
        }
    }
}
