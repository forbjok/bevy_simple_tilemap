use bevy::{
    math::{ivec3, uvec2},
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
                        resolution: WindowResolution::new(1280, 720).with_scale_factor_override(1.0),
                        ..Default::default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(SimpleTileMapPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, despawn_respawn_system)
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
            tf.scale -= Vec3::splat(ZOOM_SPEED) * time.delta_secs();
        } else if keyboard_input.pressed(KeyCode::KeyZ) {
            tf.scale += Vec3::splat(ZOOM_SPEED) * time.delta_secs();
        }

        if keyboard_input.pressed(KeyCode::KeyA) {
            tf.translation.x -= MOVE_SPEED * time.delta_secs();
        } else if keyboard_input.pressed(KeyCode::KeyD) {
            tf.translation.x += MOVE_SPEED * time.delta_secs();
        }

        if keyboard_input.pressed(KeyCode::KeyS) {
            tf.translation.y -= MOVE_SPEED * time.delta_secs();
        } else if keyboard_input.pressed(KeyCode::KeyW) {
            tf.translation.y += MOVE_SPEED * time.delta_secs();
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

fn setup(mut commands: Commands) {
    // Spawn camera
    commands.spawn(Camera2d::default());
}

fn despawn_respawn_system(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut respawn_timer: Local<Option<Timer>>,
    mut tilemap_entity: Local<Option<Entity>>,
    mut offset: Local<f32>,
    time: Res<Time>,
) {
    if respawn_timer.is_none() {
        *respawn_timer = Some(Timer::from_seconds(2., TimerMode::Repeating));
    }

    let Some(respawn_timer) = respawn_timer.as_mut() else {
        return;
    };

    respawn_timer.tick(time.delta());

    if respawn_timer.is_finished()
        && let Some(tilemap_entity) = tilemap_entity.take()
    {
        commands.entity(tilemap_entity).despawn();
    }

    if tilemap_entity.is_none() {
        *offset += 10.;

        // Load tilesheet texture and make a texture atlas from it
        let image = asset_server.load("textures/tilesheet.png");
        let atlas = TextureAtlasLayout::from_grid(uvec2(16, 16), 4, 1, Some(uvec2(1, 1)), None);
        let atlas_handle = texture_atlases.add(atlas);

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

        // Set up tilemap
        let mut tilemap = TileMap::new(image, atlas_handle);
        tilemap.set_tiles(tiles);

        // Spawn tilemap
        let entity = commands
            .spawn((
                tilemap,
                Transform {
                    scale: Vec3::splat(3.0),
                    translation: Vec3::new(*offset, 0.0, 0.0),
                    ..Default::default()
                },
            ))
            .id();

        *tilemap_entity = Some(entity);
    }
}
