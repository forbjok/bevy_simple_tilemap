use bevy::{prelude::*, reflect::TypeUuid, render2::{render_resource::{Shader, SpecializedPipelines}}};

use crate::tilemap::ChunkGpuData;

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
        fn build(&self, app: &mut App) {
            let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
            let sprite_shader = Shader::from_wgsl(include_str!("render/sprite.wgsl"));
            shaders.set_untracked(SPRITE_SHADER_HANDLE, sprite_shader);
            app.add_asset::<TextureAtlas>().register_type::<Sprite>();
            let render_app = app.sub_app(RenderApp);
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<SpritePipeline>()
                .init_resource::<SpecializedPipelines<SpritePipeline>>()
                .init_resource::<SpriteMeta>()
                .init_resource::<ExtractedSprites>()
                .add_system_to_stage(RenderStage::Extract, render::extract_sprites)
                .add_system_to_stage(RenderStage::Prepare, render::prepare_sprites)
                .add_system_to_stage(RenderStage::Queue, queue_sprites);

            let draw_sprite = DrawSprite::new(&mut render_app.world);
            render_app
                .world
                .get_resource::<DrawFunctions<Transparent2d>>()
                .unwrap()
                .write()
                .add(draw_sprite);
    }
}
