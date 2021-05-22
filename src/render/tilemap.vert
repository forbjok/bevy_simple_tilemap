#version 450

layout(location = 0) in vec2 Vertex_Position;
layout(location = 1) in uint SpriteIndex;
layout(location = 2) in vec4 SpriteColor;

layout(location = 0) out vec2 v_Uv;
layout(location = 1) out vec4 v_Color;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform TextureAtlas_size {
    vec2 AtlasSize;
};

struct Rect {
    vec2 begin;
    vec2 end;
};

layout(set = 1, binding = 1) buffer TextureAtlas_textures {
    Rect[] Textures;
};

layout(set = 2, binding = 0) uniform Transform {
    mat4 SpriteTransform;
};

void main() {
    Rect sprite_rect = Textures[SpriteIndex];

    vec2 sprite_dimensions = sprite_rect.end - sprite_rect.begin;
    vec3 vertex_position = vec3(Vertex_Position.xy * sprite_dimensions, 0.0);

    // Specify the corners of the sprite
    vec2 bottom_left = vec2(sprite_rect.begin.x, sprite_rect.end.y);
    vec2 top_left = sprite_rect.begin;
    vec2 top_right = vec2(sprite_rect.end.x, sprite_rect.begin.y);
    vec2 bottom_right = sprite_rect.end;

    vec2 atlas_positions[4] = vec2[](
        bottom_left,
        top_left,
        top_right,
        bottom_right
    );

    v_Uv = (atlas_positions[gl_VertexIndex % 4]) / AtlasSize;

    v_Color = SpriteColor;
    gl_Position = ViewProj * SpriteTransform * vec4(vertex_position, 1.0);
}
