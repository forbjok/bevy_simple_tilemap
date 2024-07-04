struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> view: View;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tile_uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

struct TilemapGpuData {
    transform: mat4x4<f32>,
    tile_size: vec2<f32>,
    texture_size: vec2<f32>,
};

@group(2) @binding(0)
var<uniform> tilemap: TilemapGpuData;

@vertex
fn vertex(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_tile_uv: vec2<f32>,
    @location(3) vertex_color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;

    out.uv = vertex_uv;
    out.tile_uv = vertex_tile_uv;
    out.position = view.view_proj * tilemap.transform * vec4<f32>(vertex_position, 1.0);
    out.color = vertex_color;

    return out;
}

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
@group(1) @binding(1)
var sprite_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let half_texture_pixel_size_u = 0.5 / tilemap.texture_size.x;
    let half_texture_pixel_size_v = 0.5 / tilemap.texture_size.y;
    let half_tile_pixel_size_u = 0.5 / tilemap.tile_size.x;
    let half_tile_pixel_size_v = 0.5 / tilemap.tile_size.y;

    // Offset the UV 1/2 pixel from the sides of the tile, so that the sampler doesn't bleed onto
    // adjacent tiles at the edges.
    var uv_offset = vec2<f32>(0.0, 0.0);

    if (in.tile_uv.x < half_tile_pixel_size_u) {
        uv_offset.x = half_texture_pixel_size_u;
    } else if (in.tile_uv.x > (1.0 - half_tile_pixel_size_u)) {
        uv_offset.x = -half_texture_pixel_size_u;
    }

    if (in.tile_uv.y < half_tile_pixel_size_v) {
        uv_offset.y = half_texture_pixel_size_v;
    } else if (in.tile_uv.y > (1.0 - half_tile_pixel_size_v)) {
        uv_offset.y = -half_texture_pixel_size_v;
    }

    var color = in.color * textureSample(sprite_texture, sprite_sampler, in.uv + uv_offset);

    return color;
}
