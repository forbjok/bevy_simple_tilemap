use bevy::{prelude::*, render::{render_resource::{Shader, SpecializedPipelines}, RenderStage, render_phase::DrawFunctions, RenderApp}, core_pipeline::Transparent2d};

use crate::{render::{TilemapPipeline, ImageBindGroups, ExtractedTilemaps, TilemapMeta, DrawTilemap, TilemapAssetEvents, TILEMAP_SHADER_HANDLE}};

#[derive(Default)]
pub struct SimpleTileMapPlugin;

#[derive(Clone, Debug, Eq, Hash, PartialEq, StageLabel)]
enum SimpleTileMapStage {
    Update,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum TilemapSystem {
    ExtractTilemap,
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
        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<ImageBindGroups>()
            .init_resource::<TilemapPipeline>()
            .init_resource::<SpecializedPipelines<TilemapPipeline>>()
            .init_resource::<TilemapMeta>()
            .init_resource::<ExtractedTilemaps>()
            .init_resource::<TilemapAssetEvents>()
            .add_system_to_stage(
                RenderStage::Extract,
                crate::render::extract_tilemaps.label(TilemapSystem::ExtractTilemap),
            )
            .add_system_to_stage(RenderStage::Extract, crate::render::extract_tilemap_events)
            .add_system_to_stage(RenderStage::Prepare, crate::render::prepare_tilemaps)
            .add_system_to_stage(RenderStage::Queue, crate::render::queue_tilemaps);

        let draw_sprite = DrawTilemap::new(&mut render_app.world);
        render_app
            .world
            .get_resource::<DrawFunctions<Transparent2d>>()
            .unwrap()
            .write()
            .add(draw_sprite);
    }
}
