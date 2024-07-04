use bevy::{
    math::{ivec2, uvec2},
    prelude::*,
    window::WindowResolution,
};

use bevy_simple_tilemap::prelude::*;

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
        .add_systems(FixedUpdate, (update_tiles1_system, update_tiles2_system))
        .insert_resource(Time::<Fixed>::from_seconds(0.05))
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

/// Draw line of foreground (layer 1) tiles, moving it up by one each time
fn update_tiles1_system(mut query: Query<(&mut TileMap, &mut TileMap1)>) {
    for (mut tilemap, state) in query.iter_mut() {
        let state = state.into_inner();
        let pos = &mut state.pos;
        let sprite_index = &mut state.sprite_index;

        // Perform tile update
        tilemap.set_tile(
            pos.extend(1),
            Some(Tile {
                sprite_index: *sprite_index,
                ..Default::default()
            }),
        );

        pos.x += 1;
        if pos.x > 9 {
            pos.x = 0;
            pos.y += 1;

            if pos.y > 9 {
                pos.y = 0;

                *sprite_index += 1;
                if *sprite_index > 3 {
                    *sprite_index = 0;
                }
            }
        }
    }
}

/// Draw line of foreground (layer 1) tiles, moving it up by one each time
fn update_tiles2_system(mut query: Query<(&mut TileMap, &mut TileMap2)>) {
    for (mut tilemap, state) in query.iter_mut() {
        let state = state.into_inner();
        let pos = &mut state.pos;
        let direction = &mut state.direction;
        let level = &mut state.level;

        if *level > 4 {
            tilemap.clear_layer(1);
            *pos = ivec2(0, 0);
            *level = 0;
        }

        // Perform tile update
        tilemap.set_tile(
            pos.extend(1),
            Some(Tile {
                sprite_index: 3,
                ..Default::default()
            }),
        );

        match *direction {
            0 => {
                pos.x += 1;

                if pos.x > (8 - *level) {
                    *direction += 1;
                }
            }
            1 => {
                pos.y += 1;

                if pos.y > (8 - *level) {
                    *direction += 1;
                }
            }
            2 => {
                pos.x -= 1;

                if pos.x < (1 + *level) {
                    *direction += 1;
                }
            }
            3 => {
                pos.y -= 1;

                if pos.y < (2 + *level) {
                    *direction = 0;
                    *level += 1;
                }
            }
            _ => {}
        };
    }
}

#[derive(Component)]
struct TileMap1 {
    pos: IVec2,
    sprite_index: u32,
}

#[derive(Component)]
struct TileMap2 {
    pos: IVec2,
    direction: u8,
    level: i32,
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

    let mut tiles = Vec::new();

    // Background (layer 0) tiles
    for y in 0..10 {
        for x in 0..10 {
            tiles.push((
                IVec3::new(x, y, 0),
                Some(Tile {
                    sprite_index: 2,
                    ..Default::default()
                }),
            ));
        }
    }

    // Set up tilemap
    let tilemap_bundle1 = {
        let mut tilemap = TileMap::default();
        tilemap.set_tiles(tiles.clone());

        TileMapBundle {
            tilemap,
            texture: texture.clone(),
            atlas: TextureAtlas {
                layout: texture_atlas.clone(),
                ..Default::default()
            },
            transform: Transform {
                scale: Vec3::splat(3.0),
                translation: Vec3::new(-500.0, -(16.0 * 3.0 * 10.0 / 2.0), 0.0),
                ..Default::default()
            },
            ..Default::default()
        }
    };

    let tilemap_bundle2 = {
        let mut tilemap = TileMap::default();
        tilemap.set_tiles(tiles);

        TileMapBundle {
            tilemap,
            texture,
            atlas: TextureAtlas {
                layout: texture_atlas,
                ..Default::default()
            },
            transform: Transform {
                scale: Vec3::splat(3.0),
                translation: Vec3::new(60.0, -(16.0 * 3.0 * 10.0 / 2.0), 0.0),
                ..Default::default()
            },
            ..Default::default()
        }
    };

    // Spawn camera
    commands.spawn(Camera2dBundle::default());

    // Spawn tilemaps
    commands.spawn(tilemap_bundle1).insert(TileMap1 {
        pos: ivec2(0, 0),
        sprite_index: 0,
    });
    commands.spawn(tilemap_bundle2).insert(TileMap2 {
        pos: ivec2(0, 0),
        direction: 0,
        level: 0,
    });
}
