# Rendering

The renderer draws exactly two kinds of geometry — depth-only **occluder
triangles** and additive **line segments** — then runs a post chain that
turns hot lines into phosphor glow. Everything below is `vex-render`.

## Frame recipe

```
1. CameraBinding.update()        one uniform: view-proj, viewport,
                                 line width (px), fog, eye, time, glow
2. OccluderRenderer              depth-only prepass → Depth32Float
                                 (clears depth; writes NO color)
3. LineRenderer                  into the HDR target, depth-tested
                                 against step 2 (clears color)
4. PostProcessor.run()           bloom → tonemap → CRT → swapchain
```

Hidden-line removal falls out of the depth test: a line behind a surface
fails against the prepass depth and disappears, with no visibility
algorithm anywhere.

### Occluder pass

No fragment shader, no color targets. A depth bias
(`constant 2`, `slope_scale 2.0`) pushes surfaces slightly *away* from
the camera so the model's own outlines — which lie exactly on those
surfaces — win the depth test instead of z-fighting.

### Line pass

Segments are instanced: one quad per segment, expanded in **screen
space** to `line_width_px` (carried in the camera uniform, so lines keep
near-constant pixel width at any distance). The fragment shader evaluates
a capsule SDF for round caps and analytic anti-aliasing (`coverage =
clamp(0.5·width − dist + 0.5, 0, 1)`), and blends **additively** —
crossing lines get hotter, which is most of the CRT feel.

Per-segment style comes from [`Segment`](../crates/vex-core/src/lib.rs):
linear RGB in `color.xyz`, a brightness multiplier in `color.w`
(dim < 1.0 < glow), `dash_period` in world units (0 = solid), and
`flicker` 0..1 (per-instance phase, driven by `time`). `fog_density`
applies an exponential depth cue: `brightness *= exp(-density · dist)`.

## HDR and the post chain

Lines render into an offscreen **RGBA16F** target (`HDR_FORMAT`), so
authored emissive strengths above 1.0 survive until post. The chain:

1. **Bloom** — threshold-less: the HDR image is downsampled through a mip
   chain and re-accumulated upward, so *everything* glows a little and
   hot things glow a lot. No bloom threshold pop.
2. **Tonemap** — `1 − exp(−x · exposure)`: filmic-ish soft shoulder,
   never clips.
3. **CRT** (dial 0..1) — barrel distortion, chromatic offset, vignette.
4. **sRGB encode, only when needed** — WebGPU swapchains have *no* sRGB
   surface formats, so when the output format is linear the composite
   shader applies the transfer function itself. Native sRGB swapchains
   skip this. Forgetting this distinction is invisible on native and
   washes out the whole web build.

### PostSettings — the look, in four numbers

| Field | Default | Meaning |
|-------|---------|---------|
| `exposure` | `1.0` | tonemap exposure |
| `bloom_strength` | `0.14` | how much of the bloom pyramid mixes in |
| `glow` | `0.5` | HDR compression dial: `0` = no overbright at all, `1` = authored emissive strengths verbatim, `>1` = hotter than authored |
| `crt` | `0.0` | 0 clean … 1 full barrel/chroma/vignette |

Scene files carry a `post:` block with the same fields, and the examples
bind live keys (`[ ] - = 9 0 C`) to them. House style: **keep it dim** —
assets author relative brightness, and these four numbers decide what
reaches the eye.

## Contracts to respect

- `CameraUniform` (Rust) and `camera.wgsl` (WGSL) describe the same
  bytes and must change **in lockstep**. Same for `PostParams` and
  `post.wgsl`. These files are deliberately guarded in this repo — treat
  any edit as a paired edit.
- The line and occluder renderers also expose `render_ranges(..)` so a
  caller can draw subsets (the scene renderer frustum-culls per instance
  by AABB and draws only visible ranges).

## Headless rendering

`Gpu::headless()` + `HeadlessTarget` render the identical pipeline into a
CPU-readable texture (`HEADLESS_FORMAT`) — this powers the `--screenshot`
flags and makes "render a frame, look at the PNG" a first-class
verification loop with no window or display server.
