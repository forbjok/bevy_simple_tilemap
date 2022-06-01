use std::ops::Range;

use bevy::{
    math::vec2,
    prelude::*,
    render::camera::{ActiveCamera, Camera2d},
};

use bevy_simple_tilemap::prelude::*;

fn main() {
    App::new()
        // Disable MSAA, as it produces weird rendering artifacts
        .insert_resource(Msaa { samples: 1 })
        .add_plugins(DefaultPlugins)
        .add_plugin(SimpleTileMapPlugin)
        .add_system(update_tiles_system)
        .add_system(input_system)
        .add_startup_system(setup)
        .run();
}

fn input_system(
    active_camera: Res<ActiveCamera<Camera2d>>,
    mut camera_transform_query: Query<(&mut Transform,), With<Camera2d>>,
    mut tilemap_visible_query: Query<&mut Visibility, With<TileMap>>,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    const MOVE_SPEED: f32 = 1000.0;
    const ZOOM_SPEED: f32 = 10.0;

    if let Some(active_camera_entity) = active_camera.get() {
        if let Ok((mut tf,)) = camera_transform_query.get_mut(active_camera_entity) {
            if keyboard_input.pressed(KeyCode::X) {
                tf.scale -= Vec3::splat(ZOOM_SPEED) * time.delta_seconds();
            } else if keyboard_input.pressed(KeyCode::Z) {
                tf.scale += Vec3::splat(ZOOM_SPEED) * time.delta_seconds();
            }

            if keyboard_input.pressed(KeyCode::A) {
                tf.translation.x -= MOVE_SPEED * time.delta_seconds();
            } else if keyboard_input.pressed(KeyCode::D) {
                tf.translation.x += MOVE_SPEED * time.delta_seconds();
            }

            if keyboard_input.pressed(KeyCode::S) {
                tf.translation.y -= MOVE_SPEED * time.delta_seconds();
            } else if keyboard_input.pressed(KeyCode::W) {
                tf.translation.y += MOVE_SPEED * time.delta_seconds();
            }

            if keyboard_input.just_pressed(KeyCode::V) {
                // Toggle visibility
                let mut visible = tilemap_visible_query.iter_mut().next().unwrap();
                visible.is_visible = !visible.is_visible;
            }
        }
    }
}

fn update_tiles_system(mut query: Query<&mut TileMap>, mut count: Local<u32>) {
    const WIDTH: i32 = 1024;
    const HEIGHT: i32 = 1024;

    const X_RANGE: Range<i32> = -(WIDTH / 2)..(WIDTH / 2);
    const Y_RANGE: Range<i32> = -(HEIGHT / 2)..(HEIGHT / 2);

    *count += 1;

    for mut tilemap in query.iter_mut() {
        // List to store set tile operations
        let mut tiles: Vec<(IVec3, Option<Tile>)> = Vec::with_capacity((WIDTH * HEIGHT) as usize);

        let mut i = *count % 4;

        for y in Y_RANGE {
            let sprite_index = i % 4;

            for x in X_RANGE {
                // Add tile change to list
                tiles.push((
                    IVec3::new(x, y, 0),
                    Some(Tile {
                        sprite_index,
                        ..Default::default()
                    }),
                ));
            }

            i += 1;
        }

        // Perform tile update
        tilemap.set_tiles(tiles);
    }
}

fn setup(asset_server: Res<AssetServer>, mut commands: Commands, mut texture_atlases: ResMut<Assets<TextureAtlas>>) {
    // Load tilesheet texture and make a texture atlas from it
    let texture_handle = asset_server.load("textures/tilesheet.png");
    let texture_atlas =
        TextureAtlas::from_grid_with_padding(texture_handle, vec2(16.0, 16.0), 4, 1, vec2(1.0, 1.0), Vec2::ZERO);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    // Set up tilemap
    let tilemap_bundle = TileMapBundle {
        texture_atlas: texture_atlas_handle.clone(),
        transform: Transform {
            scale: Vec3::splat(1.0),
            translation: Vec3::new(-640.0, -360.0, 0.0),
            ..Default::default()
        },
        ..Default::default()
    };

    // Spawn camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    // Spawn tilemap
    commands.spawn_bundle(tilemap_bundle);
}
