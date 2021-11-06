use bevy::{
    math::ivec3,
    prelude::*,
    render::camera::{ActiveCameras, Camera},
};

use bevy_simple_tilemap::{prelude::*, TileFlags};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(SimpleTileMapPlugin)
        .add_system(input_system.system())
        .add_startup_system(setup.system())
        .run();
}

fn input_system(
    active_cameras: Res<ActiveCameras>,
    mut camera_transform_query: Query<(&mut Transform,), With<Camera>>,
    mut tilemap_visible_query: Query<&mut Visible, With<TileMap>>,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    const MOVE_SPEED: f32 = 1000.0;
    const ZOOM_SPEED: f32 = 10.0;

    if let Some(active_camera_entity) = active_cameras.get("Camera2d").and_then(|ac| ac.entity) {
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

fn setup(asset_server: Res<AssetServer>, mut commands: Commands, mut texture_atlases: ResMut<Assets<TextureAtlas>>) {
    // Load tilesheet texture and make a texture atlas from it
    let texture_handle = asset_server.load("textures/tilesheet.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(16.0, 16.0), 4, 1);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    let tiles = vec![
        (
            ivec3(-4, 0, 0),
            Some(Tile {
                sprite_index: 0,
                ..Default::default()
            }),
        ),
        (
            ivec3(-3, 0, 0),
            Some(Tile {
                sprite_index: 0,
                flags: TileFlags::FLIP_X,
                ..Default::default()
            }),
        ),
        (
            ivec3(-2, 0, 0),
            Some(Tile {
                sprite_index: 1,
                ..Default::default()
            }),
        ),
        (
            ivec3(-1, 0, 0),
            Some(Tile {
                sprite_index: 1,
                flags: TileFlags::FLIP_X,
                ..Default::default()
            }),
        ),
        (
            ivec3(0, 0, 0),
            Some(Tile {
                sprite_index: 2,
                ..Default::default()
            }),
        ),
        (
            ivec3(1, 0, 0),
            Some(Tile {
                sprite_index: 2,
                flags: TileFlags::FLIP_X,
                ..Default::default()
            }),
        ),
        (
            ivec3(2, 0, 0),
            Some(Tile {
                sprite_index: 3,
                ..Default::default()
            }),
        ),
        (
            ivec3(3, 0, 0),
            Some(Tile {
                sprite_index: 3,
                flags: TileFlags::FLIP_X,
                ..Default::default()
            }),
        ),
        // Y-flipped row
        (
            ivec3(-4, 1, 0),
            Some(Tile {
                sprite_index: 0,
                flags: TileFlags::FLIP_Y,
                ..Default::default()
            }),
        ),
        (
            ivec3(-3, 1, 0),
            Some(Tile {
                sprite_index: 0,
                flags: TileFlags::FLIP_X | TileFlags::FLIP_Y,
                ..Default::default()
            }),
        ),
        (
            ivec3(-2, 1, 0),
            Some(Tile {
                sprite_index: 1,
                flags: TileFlags::FLIP_Y,
                ..Default::default()
            }),
        ),
        (
            ivec3(-1, 1, 0),
            Some(Tile {
                sprite_index: 1,
                flags: TileFlags::FLIP_X | TileFlags::FLIP_Y,
                ..Default::default()
            }),
        ),
        (
            ivec3(0, 1, 0),
            Some(Tile {
                sprite_index: 2,
                flags: TileFlags::FLIP_Y,
                ..Default::default()
            }),
        ),
        (
            ivec3(1, 1, 0),
            Some(Tile {
                sprite_index: 2,
                flags: TileFlags::FLIP_X | TileFlags::FLIP_Y,
                ..Default::default()
            }),
        ),
        (
            ivec3(2, 1, 0),
            Some(Tile {
                sprite_index: 3,
                flags: TileFlags::FLIP_Y,
                ..Default::default()
            }),
        ),
        (
            ivec3(3, 1, 0),
            Some(Tile {
                sprite_index: 3,
                flags: TileFlags::FLIP_X | TileFlags::FLIP_Y,
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
