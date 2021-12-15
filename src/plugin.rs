use bevy::{prelude::*, reflect::TypeUuid, render::{render_resource::{Shader, SpecializedPipelines}, RenderStage, render_phase::DrawFunctions, RenderApp}, core_pipeline::Transparent2d};

use crate::{tilemap::ChunkGpuData, render::{TilemapPipeline, ImageBindGroups, ExtractedTilemaps, TilemapMeta, DrawTilemap}};

#[derive(Default)]
pub struct SimpleTileMapPlugin;

#[derive(Clone, Debug, Eq, Hash, PartialEq, StageLabel)]
enum SimpleTileMapStage {
    Update,
    Remesh,
}

pub const TILEMAP_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 8852463601721108623);

impl Plugin for SimpleTileMapPlugin {
    fn build(&self, app: &mut App) {
        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        let sprite_shader = Shader::from_wgsl(include_str!("render/tilemap.wgsl"));
        shaders.set_untracked(TILEMAP_SHADER_HANDLE, sprite_shader);
        app.add_asset::<TextureAtlas>().register_type::<Sprite>();
        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<ImageBindGroups>()
            .init_resource::<TilemapPipeline>()
            .init_resource::<SpecializedPipelines<TilemapPipeline>>()
            .init_resource::<TilemapMeta>()
            .init_resource::<ExtractedTilemaps>()
            .add_system_to_stage(RenderStage::Extract, crate::render::extract_tilemaps)
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
