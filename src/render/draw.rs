use super::*;
use bevy::asset::Handle;
use bevy::ecs::system::lifetimeless::*;
use bevy::ecs::system::SystemParamItem;
use bevy::render::render_phase::{
    BatchedPhaseItem, EntityRenderCommand, RenderCommand, RenderCommandResult, SetItemPipeline,
};
use bevy::render::{render_phase::TrackedRenderPass, view::ViewUniformOffset};

pub type DrawTilemap = (
    SetItemPipeline,
    SetTilemapViewBindGroup<0>,
    SetTilemapTextureBindGroup<1>,
    SetTilemapTileGpuDataBindGroup<2>,
    SetVertexBuffer,
    DrawTilemapBatch,
);

pub struct SetTilemapViewBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetTilemapViewBindGroup<I> {
    type Param = (SRes<TilemapMeta>, SQuery<Read<ViewUniformOffset>>);

    fn render<'w>(
        view: Entity,
        _item: Entity,
        (tilemap_meta, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let view_uniform = view_query.get(view).unwrap();
        pass.set_bind_group(
            I,
            tilemap_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );

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
        let tilemap_batch = query_batch.get(item).unwrap();
        let image_bind_groups = image_bind_groups.into_inner();

        let bind_group = image_bind_groups
            .values
            .get(&Handle::weak(tilemap_batch.image_handle_id))
            .unwrap();

        pass.set_bind_group(I, bind_group, &[]);

        RenderCommandResult::Success
    }
}

pub struct SetTilemapTileGpuDataBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetTilemapTileGpuDataBindGroup<I> {
    type Param = (SRes<TilemapMeta>, SQuery<Read<TilemapBatch>>);

    fn render<'w>(
        _view: Entity,
        item: Entity,
        (tilemap_meta, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let tilemap_batch = query_batch.get(item).unwrap();
        let chunk_meta = tilemap_meta.into_inner().chunks.get(&tilemap_batch.chunk_key).unwrap();

        pass.set_bind_group(I, chunk_meta.tilemap_gpu_data_bind_group.as_ref().unwrap(), &[0]);

        RenderCommandResult::Success
    }
}

pub struct SetVertexBuffer;
impl EntityRenderCommand for SetVertexBuffer {
    type Param = (SRes<TilemapMeta>, SQuery<Read<TilemapBatch>>);

    fn render<'w>(
        _view: Entity,
        item: Entity,
        (tilemap_meta, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let tilemap_batch = query_batch.get(item).unwrap();
        let chunk_meta = tilemap_meta.into_inner().chunks.get(&tilemap_batch.chunk_key).unwrap();

        pass.set_vertex_buffer(0, chunk_meta.vertices.buffer().unwrap().slice(..));

        RenderCommandResult::Success
    }
}

pub struct DrawTilemapBatch;
impl<P: BatchedPhaseItem> RenderCommand<P> for DrawTilemapBatch {
    type Param = ();

    fn render<'w>(
        _view: Entity,
        item: &P,
        (): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.draw(item.batch_range().as_ref().unwrap().clone(), 0..1);

        RenderCommandResult::Success
    }
}
