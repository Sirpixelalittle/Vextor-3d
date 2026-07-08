# Architecture

vector3d is a from-scratch engine for one aesthetic: everything visible is
a glowing stroke, as if drawn by a vector CRT. That single idea drives the
whole architecture — surfaces exist, but only to *hide lines*, never to be
seen.

## Crate map

```
crates/
  vex-core      shared primitives: Segment, VecModel (.vec I/O), Frustum,
                vector font, phosphor colors, procedural shapes
  vex-convert   content pipeline: glTF (or any SourceGeometry) → weld →
                edge classification → palette → .vec       [lib + CLI]
  vex-render    wgpu renderer: camera, occluder depth pass, instanced line
                pass, HDR bloom + tonemap + CRT post, headless targets
  vex-engine    application shell: window/event loop, Input, cameras
                (fps / fly / orbit), capsule collision, animation clips,
                scene loading
  vex-audio     content-free spatial audio (kira) + the synth toolkit;
                games own their sound banks
```

Dependency direction is strictly downward: `vex-core` depends on nothing
in the workspace; games depend on whichever crates they need. No crate
knows about any game.

## Data flow

```
   Blender / tools/gen_*.py            05-editor
            │ glTF                        │ SourceGeometry
            ▼                             ▼
        vex-convert:  weld → classify edges → build palette
            │
            ▼  .vec  (palette, vertices, edges + face normals,
            │         occluder triangles, AABB — see formats.md)
            ▼
        vex-core: VecModel
            │ edge_segments() / silhouette_segments()     │ occluder tris
            ▼                                             ▼
        per-frame Vec<Segment>  ──────────────►  vex-render frame:
                                                 1. occluder depth prepass
                                                 2. line pass (depth-tested)
                                                 3. bloom → tonemap → CRT
            ▲
        vex-engine: run() drives App::update/render; Input, cameras,
        slide_capsule collision, AnimPlayer, BakedScene
```

The renderer consumes only two things: **segments** (world-space colored
lines) and **occluder triangles** (depth-only geometry). Everything a game
draws reduces to those, whether it comes from a baked scene, a posed
model, or code building darts and muzzle flashes segment by segment.

## The rules the engine lives by

These are working laws, learned the hard way (details in
[DESIGN.md](../DESIGN.md)):

1. **Sim and render stay separate.** Game rules live in pure, GPU-free
   modules (`04-arena/src/game.rs`) that unit-test in milliseconds; the
   binary's `main.rs` only turns state into segments. If a feature can be
   expressed as "state in, segments out", it must be.

2. **Nothing teleports.** Every position change for anything with
   collision goes through `slide_capsule` — including knockbacks and
   separation shoves. A raw `pos +=` can cross a wall plane, and the
   slide's tie-break will then keep the object on the wrong side forever.

3. **Occluders are invisible.** The depth prepass writes no color. If a
   surface shows up, it's a bug in the pass order, not a style choice.

4. **Assets author *relative* brightness; the engine decides what reaches
   the eye.** Emissive strengths in models are ratios. The post chain's
   glow dial, exposure and bloom map them to output — and the default
   look errs dim.

5. **The engine ships machinery, games ship content.** Sound recipes,
   enemy stats, HUD layout — game-side. The line between `crates/` and
   `examples/` is the line between "how" and "what".

## Where things run

Everything builds for native (Linux/macOS/Windows) and wasm/WebGPU from
the same code. The differences are confined to: surface formats (the web
has no sRGB swapchain — see [Rendering](rendering.md)), audio startup (a
user gesture must precede the AudioContext), and asset loading (embedded
on web, disk-first with embedded fallback on native).
