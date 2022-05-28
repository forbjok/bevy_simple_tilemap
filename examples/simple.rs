use bevy::{
    math::{ivec3, uvec2},
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

fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
    // Load tilesheet texture and make a texture atlas from it
    let texture = asset_server.load("textures/tilesheet.png");

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

    let mut tilemap = TileMap::new(uvec2(16, 16), UVec2::splat(1));
    tilemap.set_tiles(tiles);

    // Set up tilemap
    let tilemap_bundle = TileMapBundle {
        tilemap,
        texture,
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
