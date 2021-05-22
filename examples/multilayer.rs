use std::time::Instant;

use bevy::{core::FixedTimestep, prelude::*};

use bevy_simple_tilemap::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(SimpleTileMapPlugin)
        .add_system(update_tiles_system.system().with_run_criteria(FixedTimestep::step(0.5)))
        .add_startup_system(setup.system())
        .run();
}

/// Draw line of foreground (layer 0) tiles, moving it up by one each time
fn update_tiles_system(mut query: Query<&mut TileMap>, mut count: Local<i32>) {
    let upd_tiles = Instant::now();

    let line_y = *count % 10;

    for mut tilemap in query.iter_mut() {
        // List to store set tile operations
        let mut tiles = Vec::new();

        for y in 0..10 {
            let tile = if y == line_y {
                // Set to wall tile if line is current
                Some(Tile {
                    sprite_index: 0,
                    ..Default::default()
                })
            } else {
                // Remove tile if this line is not the current
                None
            };

            for x in 0..10 {
                // Add tile change
                tiles.push((IVec3::new(x, y, 0), tile.clone()));
            }
        }

        // Perform tile update
        tilemap.set_tiles(tiles);
    }

    dbg!(upd_tiles.elapsed());

    *count += 1;
}

fn setup(asset_server: Res<AssetServer>, mut commands: Commands, mut texture_atlases: ResMut<Assets<TextureAtlas>>) {
    // Load tilesheet texture and make a texture atlas from it
    let texture_handle = asset_server.load("textures/tilesheet.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(16.0, 16.0), 4, 1);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    let mut tiles = Vec::new();

    // Background (layer -1) tiles
    for y in 0..10 {
        for x in 0..10 {
            tiles.push((
                IVec3::new(x, y, -1),
                Some(Tile {
                    sprite_index: 2,
                    ..Default::default()
                }),
            ));
        }
    }

    let mut tilemap = TileMap::default();
    tilemap.set_tiles(tiles);

    // Set up tilemap
    let tilemap_bundle = TileMapBundle {
        tilemap,
        texture_atlas: texture_atlas_handle.clone(),
        transform: Transform {
            scale: Vec3::splat(3.0),
            translation: Vec3::new(-(16.0 * 3.0 * 10.0 / 2.0), -(16.0 * 3.0 * 10.0 / 2.0), 0.0),
            ..Default::default()
        },
        ..Default::default()
    };

    // Spawn camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    // Spawn tilemap
    commands.spawn_bundle(tilemap_bundle);
}
