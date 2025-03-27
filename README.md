# Multiply or Release

Multiply or Release is a Marble Race-like non-interactive simulation game.

## Showcase

https://github.com/user-attachments/assets/b6568097-09f6-4d65-9571-ae0c628ff452

### Rules

- Marbles race down an obstacle course to land in one of a few trigger zones. Each marble has a turret associated in the main battlefield. When the marble lands in a trigger zone, its associated turret performs the corresponding action.
- Each turret holds a charge. Depending on the zone its associated marbles land in, it can:
  - Multiply its current charge by 2 or 4.
  - Release its charge in a single powerful shot or a stream of smaller shots.
- The battlefield is made up of a grid of tiles. Each tile is associated with a turret. When a shot hits a tile for an opposing side, it consumes a charge to convert the tile.
- When a shot hits a turret, the shot and the turret each consumes an equal amount of charge. If the turret's charge goes to 0 in this exchange, it dies.

## How to Run

### Building Locally

1. Install Rust and Cargo by following the [Rust Getting Started Guide](https://www.rust-lang.org/learn/get-started).
2. ```cargo install --git https://github.com/maybe-raven/multiply-or-release```
3. Run `multiply_or_release`

> [!Warning]
> I only have a MacBook so it's only tested on MacOS. I have no idea how well it'll fare on other operating systems.

### Itch.io

You can find this game on Itch.io [here](https://maybe-raven.itch.io/multiply-or-release).

> [!Note]
> **About Particle Effects**
> 
> This game uses [`bevy_hanabi`](https://crates.io/crates/bevy_hanabi/0.12.2) for particle effects. However, the stable version of `bevy_hanabi` at the time didn't support WASM yet, so the web build will not have pretty visuals.

I think they do have WASM support now. I'll update it at some point.

## License

Licensed under either of

- MIT License ([`LICENSE-MIT`](./LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([`LICENSE-APACHE2`](./LICENSE-APACHE2) or <http://www.apache.org/licenses/LICENSE-2.0>)
