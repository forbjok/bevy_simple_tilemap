use bevy::core_pipeline::core_2d::CORE_2D_DEPTH_FORMAT;
use bevy::ecs::prelude::*;
use bevy::image::BevyDefault;
use bevy::mesh::VertexBufferLayout;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, uniform_buffer};
use bevy::render::render_resource::*;
use bevy::render::view::ViewUniform;

use super::*;

#[derive(Resource)]
pub struct TilemapPipeline {
    pub(super) view_layout: BindGroupLayoutDescriptor,
    pub(super) material_layout: BindGroupLayoutDescriptor,
    pub(super) tilemap_gpu_data_layout: BindGroupLayoutDescriptor,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

    #[inline]
    pub const fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = (msaa_samples.trailing_zeros() & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        Self::from_bits_retain(msaa_bits)
    }

    #[inline]
    pub const fn msaa_samples(&self) -> u32 {
        1 << ((self.bits() >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS)
    }
}

impl FromWorld for TilemapPipeline {
    fn from_world(_world: &mut World) -> Self {
        let view_layout = BindGroupLayoutDescriptor::new(
            "tilemap_view_layout",
            &BindGroupLayoutEntries::single(ShaderStages::VERTEX_FRAGMENT, uniform_buffer::<ViewUniform>(true)),
        );

        let material_layout = BindGroupLayoutDescriptor::new(
            "tilemap_material_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let tilemap_gpu_data_layout = BindGroupLayoutDescriptor::new(
            "tilemap_tilemap_gpu_data_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (uniform_buffer::<TilemapGpuData>(true),),
            ),
        );

        Self {
            view_layout,
            material_layout,
            tilemap_gpu_data_layout,
        }
    }
}

impl SpecializedRenderPipeline for TilemapPipeline {
    type Key = TilemapPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_formats = vec![
            // Position
            VertexFormat::Float32x3,
            // UV
            VertexFormat::Float32x2,
            // Tile UV
            VertexFormat::Float32x2,
            // Color
            VertexFormat::Float32x4,
        ];

        let vertex_buffer_layout = VertexBufferLayout::from_vertex_formats(VertexStepMode::Vertex, vertex_formats);

        let shader_defs = vec![];

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: TILEMAP_SHADER_HANDLE,
                entry_point: Some("vertex".into()),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: TILEMAP_SHADER_HANDLE,
                shader_defs,
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![
                self.view_layout.clone(),
                self.material_layout.clone(),
                self.tilemap_gpu_data_layout.clone(),
            ],
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: CORE_2D_DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("tilemap_pipeline".into()),
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        }
    }
}
