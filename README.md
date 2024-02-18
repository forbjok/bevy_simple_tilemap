# bevy_simple_tilemap

[![CI](https://github.com/forbjok/bevy_simple_tilemap/actions/workflows/ci.yml/badge.svg)](https://github.com/forbjok/bevy_simple_tilemap/actions/workflows/ci.yml)
![GitHub release (latest by date)](https://img.shields.io/github/v/release/forbjok/bevy_simple_tilemap)
![Crates.io](https://img.shields.io/crates/v/bevy_simple_tilemap)

Refreshingly simple tilemap implementation for Bevy Engine.

## Why another tilemap?

The main reason I started this was because I felt the existing tilemap implementations for Bevy were needlessly complicated to use when all you want to do is to as quickly and simply as possible render a grid of tiles to the screen, often exposing internal implementation details such as chunks to the user.

## Goals:
* Allow the user to render a grid of rectangular tiles to the screen
* Make this as simple and intuitive as possible

## Non-goals:
* Supporting every imaginable shape of tile
* 3D tilemaps
* Assisting with non-rendering-related game-logic

## How to use:

### Spawning:
```rust
fn setup(
  asset_server: Res<AssetServer>,
  mut commands: Commands,
  mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    // Load tilesheet texture and make a texture atlas from it
    let texture = asset_server.load("textures/tilesheet.png");
    let atlas = TextureAtlasLayout::from_grid(vec2(16.0, 16.0), 4, 1, Some(vec2(1.0, 1.0)), None);
    let texture_atlas = texture_atlases.add(atlas);

    // Set up tilemap
    let tilemap_bundle = TileMapBundle {
        texture,
        atlas: TextureAtlas {
            layout: texture_atlas,
            ..Default::default()
        },
        ..Default::default()
    };

    // Spawn tilemap
    commands.spawn(tilemap_bundle);
}
```

### Updating (or inserting) single tile:
```rust
tilemap.set_tile(ivec3(0, 0, 0), Some(Tile { sprite_index: 0, color: Color::WHITE }));
```

### Updating (or inserting) multiple tiles:
```rust
// List to store set tile operations
let mut tiles: Vec<(IVec3, Option<Tile>)> = Vec::new();
tiles.push((ivec3(0, 0, 0), Some(Tile { sprite_index: 0, color: Color::WHITE })));
tiles.push((ivec3(1, 0, 0), Some(Tile { sprite_index: 1, color: Color::WHITE })));

// Perform tile update
tilemap.set_tiles(tiles);
```
