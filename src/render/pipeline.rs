use bevy::ecs::prelude::*;
use bevy::ecs::system::SystemState;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, uniform_buffer};
use bevy::render::view::ViewUniform;
use bevy::render::{render_resource::*, renderer::RenderDevice, texture::BevyDefault};

use super::*;

#[derive(Resource)]
pub struct TilemapPipeline {
    pub(super) view_layout: BindGroupLayout,
    pub(super) material_layout: BindGroupLayout,
    pub(super) tilemap_gpu_data_layout: BindGroupLayout,
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
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(Res<RenderDevice>,)> = SystemState::new(world);
        let (render_device,) = system_state.get_mut(world);

        let view_layout = render_device.create_bind_group_layout(
            "tilemap_view_layout",
            &BindGroupLayoutEntries::single(ShaderStages::VERTEX_FRAGMENT, uniform_buffer::<ViewUniform>(true)),
        );

        let material_layout = render_device.create_bind_group_layout(
            "tilemap_material_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let tilemap_gpu_data_layout = render_device.create_bind_group_layout(
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
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: TILEMAP_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
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
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("tilemap_pipeline".into()),
            push_constant_ranges: Vec::new(),
        }
    }
}
