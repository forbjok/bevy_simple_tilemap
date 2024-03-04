use bevy::{
    math::{ivec3, uvec2},
    prelude::*,
    window::WindowResolution,
};

use bevy_simple_tilemap::{prelude::*, TileFlags};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(1280.0, 720.0).with_scale_factor_override(1.0),
                        ..Default::default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(SimpleTileMapPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, input_system)
        .run();
}

fn input_system(
    mut camera_transform_query: Query<&mut Transform, With<Camera2d>>,
    mut tilemap_visible_query: Query<&mut Visibility, With<TileMap>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    const MOVE_SPEED: f32 = 1000.0;
    const ZOOM_SPEED: f32 = 10.0;

    if let Some(mut tf) = camera_transform_query.iter_mut().next() {
        if keyboard_input.pressed(KeyCode::KeyX) {
            tf.scale -= Vec3::splat(ZOOM_SPEED) * time.delta_seconds();
        } else if keyboard_input.pressed(KeyCode::KeyZ) {
            tf.scale += Vec3::splat(ZOOM_SPEED) * time.delta_seconds();
        }

        if keyboard_input.pressed(KeyCode::KeyA) {
            tf.translation.x -= MOVE_SPEED * time.delta_seconds();
        } else if keyboard_input.pressed(KeyCode::KeyD) {
            tf.translation.x += MOVE_SPEED * time.delta_seconds();
        }

        if keyboard_input.pressed(KeyCode::KeyS) {
            tf.translation.y -= MOVE_SPEED * time.delta_seconds();
        } else if keyboard_input.pressed(KeyCode::KeyW) {
            tf.translation.y += MOVE_SPEED * time.delta_seconds();
        }

        if keyboard_input.just_pressed(KeyCode::KeyV) {
            // Toggle visibility
            let mut visibility = tilemap_visible_query.iter_mut().next().unwrap();

            if *visibility == Visibility::Hidden {
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

fn setup(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Load tilesheet texture and make a texture atlas from it
    let texture = asset_server.load("textures/tilesheet.png");
    let atlas = TextureAtlasLayout::from_grid(uvec2(16, 16), 4, 1, Some(uvec2(1, 1)), None);
    let texture_atlas = texture_atlases.add(atlas);

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
        texture,
        atlas: TextureAtlas {
            layout: texture_atlas,
            ..Default::default()
        },
        transform: Transform {
            scale: Vec3::splat(3.0),
            translation: Vec3::new(0.0, 0.0, 0.0),
            ..Default::default()
        },
        ..Default::default()
    };

    // Spawn camera
    commands.spawn(Camera2dBundle::default());

    // Spawn tilemap
    commands.spawn(tilemap_bundle);
}
