use bevy::{prelude::*, render::{render_resource::{Shader, SpecializedPipelines}, RenderStage, render_phase::AddRenderCommand, RenderApp}, core_pipeline::Transparent2d};

use crate::{render::{TilemapPipeline, ImageBindGroups, ExtractedTilemaps, TilemapMeta, DrawTilemap, TilemapAssetEvents, TILEMAP_SHADER_HANDLE, self}};

#[derive(Default)]
pub struct SimpleTileMapPlugin;

#[derive(Clone, Debug, Eq, Hash, PartialEq, StageLabel)]
enum SimpleTileMapStage {
    Update,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum TilemapSystem {
    ExtractTilemaps,
}

impl Plugin for SimpleTileMapPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_stage_before(
            CoreStage::PostUpdate,
            SimpleTileMapStage::Update,
            SystemStage::parallel(),
        )
        .add_system_to_stage(
            SimpleTileMapStage::Update,
            crate::tilemap::update_chunks_system.system(),
        );
        /*.add_system_to_stage(
            SimpleTileMapStage::Remesh,
            crate::tilemap::remesh_chunks_system.system(),
        ); */

        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        let sprite_shader = Shader::from_wgsl(include_str!("render/tilemap.wgsl"));
        shaders.set_untracked(TILEMAP_SHADER_HANDLE, sprite_shader);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<TilemapPipeline>()
                .init_resource::<SpecializedPipelines<TilemapPipeline>>()
                .init_resource::<TilemapMeta>()
                .init_resource::<ExtractedTilemaps>()
                .init_resource::<TilemapAssetEvents>()
                .add_render_command::<Transparent2d, DrawTilemap>()
                .add_system_to_stage(
                    RenderStage::Extract,
                    render::extract_tilemaps.label(TilemapSystem::ExtractTilemaps),
                )
                .add_system_to_stage(RenderStage::Extract, render::extract_tilemap_events)
                .add_system_to_stage(RenderStage::Queue, render::queue_tilemaps);
        };
    }
}
