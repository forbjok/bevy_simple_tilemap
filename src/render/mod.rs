use bevy::asset::{Assets, HandleUntyped};
use bevy::reflect::TypeUuid;
use bevy::render::render_graph::AssetRenderResourcesNode;
use bevy::render::pipeline::BlendComponent;
use bevy::render::{
    pipeline::{
        BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrite, CompareFunction, DepthBiasState,
        DepthStencilState, FrontFace, PipelineDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
        StencilFaceState, StencilState,
    },
    render_graph::RenderGraph,
    shader::{Shader, ShaderStage, ShaderStages},
    texture::TextureFormat,
};

use crate::tilemap::ChunkGpuData;

pub const TILEMAP_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 9765236402292098257);

pub fn build_tilemap_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
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
        color_target_states: vec![ColorTargetState {
            format: TextureFormat::default(),
            blend: Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            }),
            write_mask: ColorWrite::ALL,
        }],
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: PolygonMode::Fill,
            clamp_depth: false,
            conservative: false,
        },
        ..PipelineDescriptor::new(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, include_str!("tilemap.vert"))),
            fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, include_str!("tilemap.frag")))),
        })
    }
}

pub mod node {
    pub const TILEMAP_CHUNK_GPU_DATA: &str = "tilemap_chunk_gpu_data";
}

pub(crate) fn add_tilemap_graph(
    graph: &mut RenderGraph,
    pipelines: &mut Assets<PipelineDescriptor>,
    shaders: &mut Assets<Shader>,
) {
    graph.add_system_node(
        node::TILEMAP_CHUNK_GPU_DATA,
        AssetRenderResourcesNode::<ChunkGpuData>::new(false),
    );

    pipelines.set_untracked(TILEMAP_PIPELINE_HANDLE, build_tilemap_pipeline(shaders));
}
