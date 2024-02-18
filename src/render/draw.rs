use super::*;
use bevy::ecs::system::lifetimeless::*;
use bevy::ecs::system::SystemParamItem;
use bevy::render::render_phase::PhaseItem;
use bevy::render::render_phase::{RenderCommand, RenderCommandResult, SetItemPipeline};
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
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetTilemapViewBindGroup<I> {
    type Param = SRes<TilemapMeta>;
    type ViewQuery = Read<ViewUniformOffset>;
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform: &'_ ViewUniformOffset,
        _entity: Option<()>,
        tilemap_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            tilemap_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );

        RenderCommandResult::Success
    }
}

pub struct SetTilemapTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetTilemapTextureBindGroup<I> {
    type Param = (SRes<ImageBindGroups>, SQuery<Read<TilemapBatch>>);
    type ViewQuery = ();
    type ItemQuery = Entity;

    fn render<'w>(
        _item: &P,
        _view: (),
        entity: Option<Entity>,
        (image_bind_groups, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(entity) = entity else {
            return RenderCommandResult::Failure;
        };

        let tilemap_batch = query_batch.get(entity).unwrap();
        let image_bind_groups = image_bind_groups.into_inner();

        pass.set_bind_group(
            I,
            image_bind_groups.values.get(&tilemap_batch.image_handle_id).unwrap(),
            &[],
        );

        RenderCommandResult::Success
    }
}

pub struct SetTilemapTileGpuDataBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetTilemapTileGpuDataBindGroup<I> {
    type Param = (SRes<TilemapMeta>, SQuery<Read<TilemapBatch>>);
    type ViewQuery = ();
    type ItemQuery = Entity;

    fn render<'w>(
        _item: &P,
        _view: (),
        entity: Option<Entity>,
        (tilemap_meta, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(entity) = entity else {
            return RenderCommandResult::Failure;
        };

        let tilemap_batch = query_batch.get(entity).unwrap();
        let chunk_meta = tilemap_meta.into_inner().chunks.get(&tilemap_batch.chunk_key).unwrap();

        pass.set_bind_group(I, chunk_meta.tilemap_gpu_data_bind_group.as_ref().unwrap(), &[0]);

        RenderCommandResult::Success
    }
}

pub struct SetVertexBuffer;
impl<P: PhaseItem> RenderCommand<P> for SetVertexBuffer {
    type Param = (SRes<TilemapMeta>, SQuery<Read<TilemapBatch>>);
    type ViewQuery = ();
    type ItemQuery = Entity;

    fn render<'w>(
        _item: &P,
        _view: (),
        entity: Option<Entity>,
        (tilemap_meta, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(entity) = entity else {
            return RenderCommandResult::Failure;
        };

        let tilemap_batch = query_batch.get(entity).unwrap();
        let chunk_meta = tilemap_meta.into_inner().chunks.get(&tilemap_batch.chunk_key).unwrap();

        if let Some(buffer) = chunk_meta.vertices.buffer() {
            pass.set_vertex_buffer(0, buffer.slice(..));
        }

        RenderCommandResult::Success
    }
}

pub struct DrawTilemapBatch;
impl<P: PhaseItem> RenderCommand<P> for DrawTilemapBatch {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = Read<TilemapBatch>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'_ TilemapBatch>,
        (): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batch else {
            return RenderCommandResult::Failure;
        };

        pass.draw(batch.range.clone(), 0..1);

        RenderCommandResult::Success
    }
}
