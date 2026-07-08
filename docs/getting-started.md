# Getting started

## Prerequisites

- **Rust** (stable, 2024 edition). The workspace tracks current stable.
- A GPU with **Vulkan, Metal or DX12** (anything wgpu can drive).
- Linux needs the usual windowing dev packages (X11 or Wayland) and ALSA
  for audio; macOS and Windows need nothing extra.
- Web builds have their own toolchain notes — see [Web builds](web.md).

Dependency versions are deliberately modern (`wgpu 30`, `glam 0.33`,
`kira 0.12`, `winit`); `Cargo.lock` is committed and is the source of
truth.

## Run the examples

```sh
cargo run -p cube                                   # spinning wireframe cube
cargo run -p viewer -- assets/suzanne/suzanne.vec   # model viewer
cargo run -p corridor                               # walkable scene
cargo run -p arena                                  # the wave-shooter demo
cargo run -p editor                                 # block-out level editor
```

Debug builds are fine for development (the workspace profile keeps them
optimized enough); use `--release` for real play.

### Common controls

Left click captures the mouse for look; **Escape** releases it. The window
title shows a live fps readout. In the scene-based examples:

| Key | Effect |
|-----|--------|
| `W A S D` | move |
| `Space` / `Shift` | dash (arena) |
| `[` `]` | bloom strength down / up |
| `-` `=` | exposure down / up |
| `9` `0` | glow dial down / up |
| `C` | toggle CRT post effect |

The editor has its own bindings — press **F1** in-editor for the overlay.

### Useful flags

The viewer and arena double as headless screenshot tools (they render one
frame offscreen and write a PNG — no window, works over SSH):

```sh
# Viewer: pose, animate, frame a model
cargo run -p viewer -- assets/suzanne/suzanne.vec \
    --anim assets/test/pulse-spin.anim.ron --time 1.5 \
    --screenshot out.png --size 1280x720 [--yaw D --pitch D --zoom Z --smooth]

# Arena: jump waves, simulate, screenshot
cargo run -p arena -- --wave 10                    # start at the boss wave
cargo run -p arena -- --screenshot out.png --size 1280x720 \
    --demo 8.0                                     # simulate 8s first
cargo run -p arena -- --screenshot out.png --menu [--options]  # start screen
```

The viewer also accepts `.gltf`/`.glb` directly (converted in-process) and
hot-reloads the model file and `--anim` clip when they change on disk.

## Convert your own model

```sh
cargo run -p vex-convert -- model.gltf -o model.vec [--crease 30]
cargo run -p viewer -- model.vec
```

See [File formats](formats.md) for what the converter does and the
authoring conventions (emissive colors, material-name styles).

## Tests

```sh
cargo test --workspace          # every suite; all pure-CPU, no GPU needed
cargo clippy --workspace --all-targets
cargo test -p vex-audio -- --ignored   # audible smoke test (plays tones)
```

The repository convention: tests and clippy stay green on every commit,
and rendering changes are verified by taking a headless screenshot and
*looking at it*.
