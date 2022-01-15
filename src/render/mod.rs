use std::{cmp::Ordering, ops::Range};

use bevy::ecs::system::SystemParamItem;
use bevy::prelude::{HandleUntyped, Msaa};
use bevy::reflect::{TypeUuid, Uuid};
use bevy::render::render_phase::{BatchedPhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, EntityRenderCommand};
use bevy::render::render_resource::std140::AsStd140;
use bevy::sprite::{Rect, TextureAtlas, SpritePipelineKey};
use bevy::asset::{AssetEvent, Assets, Handle, HandleId};
use bevy::core::FloatOrd;
use bevy::core_pipeline::Transparent2d;
use bevy::ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemState},
};
use bevy::math::{const_vec3, Mat4, Vec2, Vec3, Vec4Swizzles, IVec2, const_vec2};
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

        Self {
            view_layout,
            material_layout,
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    // MSAA uses the highest 6 bits for the MSAA sample count - 1 to support up to 64x MSAA.
    pub struct TilemapPipelineKey: u32 {
        const NONE                        = 0;
        const MSAA_RESERVED_BITS          = TilemapPipelineKey::MSAA_MASK_BITS << TilemapPipelineKey::MSAA_SHIFT_BITS;
    }
}

impl TilemapPipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = ((msaa_samples - 1) & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        TilemapPipelineKey::from_bits(msaa_bits).unwrap()
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }
}

impl SpecializedPipeline for TilemapPipeline {
    type Key = SpritePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut vertex_buffer_layout = VertexBufferLayout {
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
                VertexAttribute {
                    format: VertexFormat::Uint32,
                    offset: 20,
                    shader_location: 2,
                },
            ],
        };

        let mut shader_defs = Vec::new();

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
                count: key.msaa_samples(),
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
    pub transform: GlobalTransform,
    pub image_handle_id: HandleId,
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
                    transform: *transform,
                    image_handle_id: texture_atlas.texture.id,
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

#[derive(Component, Eq, PartialEq, Copy, Clone)]
pub struct TilemapBatch {
    image_handle_id: HandleId,
}

#[derive(Default)]
pub struct ImageBindGroups {
    values: HashMap<Handle<Image>, BindGroup>,
}

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
        let key = SpritePipelineKey::from_msaa_samples(msaa.samples);
        let pipeline = pipelines.specialize(&mut pipeline_cache, &tilemap_pipeline, key);

        // Vertex buffer indices
        let mut index = 0;

        // FIXME: VisibleEntities is ignored
        for mut transparent_phase in views.iter_mut() {
            let tilemaps = &mut extracted_tilemaps.tilemaps;
            let image_bind_groups = &mut *image_bind_groups;

            transparent_phase.items.reserve(tilemaps.len());

            // Sort sprites by z for correct transparency and then by handle to improve batching
            tilemaps.sort_unstable_by(|a, b| {
                match a
                    .transform
                    .translation
                    .z
                    .partial_cmp(&b.transform.translation.z)
                {
                    Some(Ordering::Equal) | None => a.image_handle_id.cmp(&b.image_handle_id),
                    Some(other) => other,
                }
            });

            for tilemap in tilemaps.iter() {
                let batch = TilemapBatch {
                    image_handle_id: tilemap.image_handle_id,
                };

                let image_size;
                let batch_entity;

                // Set-up a new possible batch
                if let Some(gpu_image) =
                    gpu_images.get(&Handle::weak(batch.image_handle_id))
                {
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
                                        resource: BindingResource::TextureView(
                                            &gpu_image.texture_view,
                                        ),
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

                for tile in tilemap.tiles.iter() {
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

                    let tile_pos = tile.pos.as_vec2() * rect_size; // TODO: Make work

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

                    let item_start = index;
                    index += QUAD_INDICES.len() as u32;
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
        }
        sprite_meta
            .vertices
            .write_buffer(&render_device, &render_queue);
    }
}

pub type DrawTilemap = (
    SetItemPipeline,
    SetTilemapViewBindGroup<0>,
    SetTilemapTextureBindGroup<1>,
    DrawTilemapBatch,
);

pub struct SetTilemapViewBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetTilemapViewBindGroup<I> {
    type Param = (SRes<TilemapMeta>, SQuery<Read<ViewUniformOffset>>);

    fn render<'w>(
        view: Entity,
        _item: Entity,
        (sprite_meta, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let view_uniform = view_query.get(view).unwrap();
        pass.set_bind_group(
            I,
            sprite_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}
pub struct SetTilemapTextureBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetTilemapTextureBindGroup<I> {
    type Param = (SRes<ImageBindGroups>, SQuery<Read<TilemapBatch>>);

    fn render<'w>(
        _view: Entity,
        item: Entity,
        (image_bind_groups, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let tilemap_batch = query_batch.get(item).unwrap();
        let image_bind_groups = image_bind_groups.into_inner();

        pass.set_bind_group(
            1,
            image_bind_groups
                .values
                .get(&Handle::weak(tilemap_batch.image_handle_id))
                .unwrap(),
            &[],
        );
        RenderCommandResult::Success
    }
}

pub struct DrawTilemapBatch;
impl<P: BatchedPhaseItem> RenderCommand<P> for DrawTilemapBatch {
    type Param = (SRes<TilemapMeta>, SQuery<Read<TilemapBatch>>);

    fn render<'w>(
        _view: Entity,
        item: &P,
        (tilemap_meta, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let tilemap_batch = query_batch.get(item.entity()).unwrap();
        let tilemap_meta = tilemap_meta.into_inner();

        pass.set_vertex_buffer(0, tilemap_meta.vertices.buffer().unwrap().slice(..));

        pass.draw(item.batch_range().as_ref().unwrap().clone(), 0..1);
        RenderCommandResult::Success
    }
}
