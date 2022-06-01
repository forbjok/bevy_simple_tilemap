use bevy::{
    math::{ivec2, vec2},
    prelude::*,
    render::camera::{ActiveCamera, Camera2d},
    time::FixedTimestep,
};

use bevy_simple_tilemap::prelude::*;

fn main() {
    App::new()
        // Disable MSAA, as it produces weird rendering artifacts
        .insert_resource(Msaa { samples: 1 })
        .add_plugins(DefaultPlugins)
        .add_plugin(SimpleTileMapPlugin)
        .add_system(update_tiles1_system.with_run_criteria(FixedTimestep::step(0.1)))
        .add_system(update_tiles2_system.with_run_criteria(FixedTimestep::step(0.05)))
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

fn setup(asset_server: Res<AssetServer>, mut commands: Commands, mut texture_atlases: ResMut<Assets<TextureAtlas>>) {
    // Load tilesheet texture and make a texture atlas from it
    let texture_handle = asset_server.load("textures/tilesheet.png");
    let texture_atlas =
        TextureAtlas::from_grid_with_padding(texture_handle, vec2(16.0, 16.0), 4, 1, vec2(1.0, 1.0), Vec2::ZERO);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

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
            texture_atlas: texture_atlas_handle.clone(),
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
            texture_atlas: texture_atlas_handle.clone(),
            transform: Transform {
                scale: Vec3::splat(3.0),
                translation: Vec3::new(60.0, -(16.0 * 3.0 * 10.0 / 2.0), 0.0),
                ..Default::default()
            },
            ..Default::default()
        }
    };

    // Spawn camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    // Spawn tilemaps
    commands.spawn_bundle(tilemap_bundle1).insert(TileMap1 {
        pos: ivec2(0, 0),
        sprite_index: 0,
    });
    commands.spawn_bundle(tilemap_bundle2).insert(TileMap2 {
        pos: ivec2(0, 0),
        direction: 0,
        level: 0,
    });
}
