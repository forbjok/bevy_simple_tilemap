use bevy::{math::ivec3, prelude::*};

use bevy_simple_tilemap::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(SimpleTileMapPlugin)
        .add_startup_system(setup.system())
        .run();
}

fn setup(asset_server: Res<AssetServer>, mut commands: Commands, mut texture_atlases: ResMut<Assets<TextureAtlas>>) {
    // Load tilesheet texture and make a texture atlas from it
    let texture_handle = asset_server.load("textures/tilesheet.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(16.0, 16.0), 4, 1);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    let tiles = vec![
        (
            ivec3(-1, 0, 0),
            Some(Tile {
                sprite_index: 0,
                ..Default::default()
            }),
        ),
        (
            ivec3(1, 0, 0),
            Some(Tile {
                sprite_index: 1,
                ..Default::default()
            }),
        ),
        (
            ivec3(0, -1, 0),
            Some(Tile {
                sprite_index: 2,
                ..Default::default()
            }),
        ),
        (
            ivec3(0, 1, 0),
            Some(Tile {
                sprite_index: 3,
                ..Default::default()
            }),
        ),
    ];

    let mut tilemap = TileMap::default();
    tilemap.set_tiles(tiles);

    // Set up tilemap
    let tilemap_bundle = TileMapBundle {
        tilemap,
        texture_atlas: texture_atlas_handle.clone(),
        transform: Transform {
            scale: Vec3::splat(3.0),
            translation: Vec3::new(0.0, 0.0, 0.0),
            ..Default::default()
        },
        ..Default::default()
    };

    // Spawn camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    // Spawn tilemap
    commands.spawn_bundle(tilemap_bundle);
}
