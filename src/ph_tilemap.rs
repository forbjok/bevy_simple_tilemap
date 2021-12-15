use bevy::ecs::component::Component;
use bevy::math::Vec2;
use bevy::reflect::{Reflect, TypeUuid};
use bevy::render::color::Color;

#[derive(Component, Debug, Default, Clone, TypeUuid, Reflect)]
#[uuid = "7233c597-ccfa-411f-bd59-9af349432ada"]
#[repr(C)]
pub struct Tilemap {
    /// The sprite's color tint
    pub color: Color,
    /// Flip the sprite along the X axis
    pub flip_x: bool,
    /// Flip the sprite along the Y axis
    pub flip_y: bool,
    /// An optional custom size for the sprite that will be used when rendering, instead of the size
    /// of the sprite's image
    pub custom_size: Option<Vec2>,
}

#[derive(Component, Debug, Clone, TypeUuid, Reflect)]
#[uuid = "7233c597-ccfa-411f-bd59-9af349432ada"]
pub struct TextureAtlasTilemap {
    pub color: Color,
    pub index: usize,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl Default for TextureAtlasTilemap {
    fn default() -> Self {
        Self {
            index: 0,
            color: Color::WHITE,
            flip_x: false,
            flip_y: false,
        }
    }
}

impl TextureAtlasTilemap {
    pub fn new(index: usize) -> TextureAtlasTilemap {
        Self {
            index,
            ..Default::default()
        }
    }
}
