use bevy::{
    core_pipeline::core_2d::Transparent2d,
    prelude::*,
    render::{
        render_phase::AddRenderCommand,
        render_resource::{Shader, SpecializedRenderPipelines},
        RenderApp, RenderSet,
    },
};

use crate::render::{
    self, draw::DrawTilemap, pipeline::TilemapPipeline, ExtractedTilemaps, ImageBindGroups, TilemapAssetEvents,
    TilemapMeta, TILEMAP_SHADER_HANDLE,
};

#[derive(Default)]
pub struct SimpleTileMapPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum TileMapSystem {
    ExtractTilemaps,
}

impl Plugin for SimpleTileMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(crate::tilemap::update_chunks_system);

        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        let sprite_shader = Shader::from_wgsl(include_str!("render/tilemap.wgsl"));
        shaders.set_untracked(TILEMAP_SHADER_HANDLE, sprite_shader);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<TilemapPipeline>()
                .init_resource::<SpecializedRenderPipelines<TilemapPipeline>>()
                .init_resource::<TilemapMeta>()
                .init_resource::<ExtractedTilemaps>()
                .init_resource::<TilemapAssetEvents>()
                .add_render_command::<Transparent2d, DrawTilemap>()
                .add_systems(
                    (
                        render::extract::extract_tilemaps.in_set(TileMapSystem::ExtractTilemaps),
                        render::extract::extract_tilemap_events,
                    )
                        .in_schedule(ExtractSchedule),
                )
                .add_system(render::queue::queue_tilemaps.in_set(RenderSet::Queue));
        };
    }
}
