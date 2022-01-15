use super::*;
use bevy::asset::Handle;
use bevy::ecs::system::SystemParamItem;
use bevy::ecs::{prelude::*, system::lifetimeless::*};
use bevy::prelude::info;
use bevy::render::render_phase::{
    BatchedPhaseItem, EntityRenderCommand, RenderCommand, RenderCommandResult, SetItemPipeline,
};
use bevy::render::{render_phase::TrackedRenderPass, view::ViewUniformOffset};
use bevy::utils::Instant;

pub type DrawTilemap = (
    SetItemPipeline,
    SetTilemapViewBindGroup<0>,
    SetTilemapTextureBindGroup<1>,
    DrawTilemapBatch,
);

pub struct SetTilemapViewBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetTilemapViewBindGroup<I> {
    type Param = (SRes<TilemapMeta>, SQuery<Read<ViewUniformOffset>>);

    fn render<'w>(
        view: Entity,
        _item: Entity,
        (sprite_meta, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let timer = Instant::now();

        let view_uniform = view_query.get(view).unwrap();
        pass.set_bind_group(
            I,
            sprite_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );

        info!("SetTilemapViewBindGroup {:?}", timer.elapsed());
        RenderCommandResult::Success
    }
}
pub struct SetTilemapTextureBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetTilemapTextureBindGroup<I> {
    type Param = (SRes<ImageBindGroups>, SQuery<Read<TilemapBatch>>);

    fn render<'w>(
        _view: Entity,
        item: Entity,
        (image_bind_groups, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let timer = Instant::now();

        let tilemap_batch = query_batch.get(item).unwrap();
        let image_bind_groups = image_bind_groups.into_inner();

        pass.set_bind_group(
            1,
            image_bind_groups
                .values
                .get(&Handle::weak(tilemap_batch.image_handle_id))
                .unwrap(),
            &[],
        );

        info!("SetTilemapTextureBindGroup {:?}", timer.elapsed());
        RenderCommandResult::Success
    }
}

pub struct DrawTilemapBatch;
impl<P: BatchedPhaseItem> RenderCommand<P> for DrawTilemapBatch {
    type Param = (SRes<TilemapMeta>, SQuery<Read<TilemapBatch>>);

    fn render<'w>(
        _view: Entity,
        item: &P,
        (tilemap_meta, _query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let timer = Instant::now();

        //let tilemap_batch = query_batch.get(item.entity()).unwrap();
        let tilemap_meta = tilemap_meta.into_inner();

        pass.set_vertex_buffer(0, tilemap_meta.vertices.buffer().unwrap().slice(..));

        pass.draw(item.batch_range().as_ref().unwrap().clone(), 0..1);

        info!("DRAW {:?}", timer.elapsed());
        RenderCommandResult::Success
    }
}
