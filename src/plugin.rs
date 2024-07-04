use bevy::{
    asset::load_internal_asset,
    core_pipeline::core_2d::Transparent2d,
    prelude::*,
    render::{
        render_phase::AddRenderCommand,
        render_resource::{Shader, SpecializedRenderPipelines},
        view::{check_visibility, VisibilitySystems},
        Render, RenderApp, RenderSet,
    },
};

use crate::{
    render::{
        self, draw::DrawTilemap, pipeline::TilemapPipeline, ExtractedTilemaps, ImageBindGroups, TilemapAssetEvents,
        TilemapMeta, TILEMAP_SHADER_HANDLE,
    },
    tilemap::WithTileMap,
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

        app.add_systems(
            PostUpdate,
            check_visibility::<WithTileMap>.in_set(VisibilitySystems::CheckVisibility),
        );

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
                .add_systems(Render, render::queue::queue_tilemaps.in_set(RenderSet::Queue));
        };
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<TilemapPipeline>();
        }
    }
}
