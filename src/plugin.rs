use bevy::{
    asset::load_internal_asset,
    core_pipeline::core_2d::Transparent2d,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        render_phase::AddRenderCommand,
        render_resource::{Shader, SpecializedRenderPipelines},
    },
};

use crate::render::{
    self, ExtractedTilemaps, ImageBindGroups, TILEMAP_SHADER_HANDLE, TilemapAssetEvents, TilemapMeta,
    draw::DrawTilemap, pipeline::TilemapPipeline,
};

#[derive(Default)]
pub struct SimpleTileMapPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum TileMapSystem {
    ExtractTilemaps,
}

impl Plugin for SimpleTileMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, crate::tilemap::update_chunks_system);

        load_internal_asset!(app, TILEMAP_SHADER_HANDLE, "render/tilemap.wgsl", Shader::from_wgsl);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<SpecializedRenderPipelines<TilemapPipeline>>()
                .init_resource::<TilemapMeta>()
                .init_resource::<ExtractedTilemaps>()
                .init_resource::<TilemapAssetEvents>()
                .add_render_command::<Transparent2d, DrawTilemap>()
                .add_systems(
                    ExtractSchedule,
                    (
                        render::extract::extract_tilemaps.in_set(TileMapSystem::ExtractTilemaps),
                        render::extract::extract_tilemap_events,
                    ),
                )
                .add_systems(Render, render::queue::queue_tilemaps.in_set(RenderSystems::Queue));
        };
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<TilemapPipeline>();
        }
    }
}
