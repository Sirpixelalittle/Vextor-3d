# Web builds

The same code runs in the browser on **WebGPU** — no WebGL fallback, no
separate code paths beyond the handful of differences listed here. The
arena demo ships this way ([play it](https://chainsawqueen1.itch.io/vector3d-arena)).

## Build

```sh
tools/build_web.sh        # → dist-web/ (arena.js + arena_bg.wasm + index.html)
```

Requirements (the script checks and tells you what's missing):

- the `wasm32-unknown-unknown` std — `rustup target add
  wasm32-unknown-unknown` (Arch: `pacman -S rust-wasm`, and keep its
  pkgrel in step with `rust` itself; a skewed pair miscompiles).
- a `wasm-bindgen` **CLI that exactly matches the `wasm-bindgen` version
  in `Cargo.lock`** — the script reads the lock and refuses politely:
  `cargo install wasm-bindgen-cli --version <locked> --locked`.

Test locally (WebGPU requires a secure context — localhost counts):

```sh
python3 -m http.server 8080 -d dist-web    # → http://localhost:8080
```

## What's different in the browser

- **No sRGB swapchain.** WebGPU surface formats are linear, so the
  composite shader applies the sRGB transfer function itself (driven by
  `PostProcessor`'s format check). Nothing to do per game — but if the
  web build ever looks washed out or too dark next to native, start here.
  Details in [Rendering](rendering.md).
- **GPU setup is async.** `run()` returns immediately on wasm; the
  adapter/device arrive later and `App::init` fires then. Don't touch the
  GPU before `init`.
- **Audio needs a user gesture.** Create the `AudioEngine` after the
  first captured click (the examples fold this into mouse capture), or
  the AudioContext stays suspended.
- **Assets are embedded.** No filesystem — the arena compiles its content
  in via `include_bytes!` and uses the same embedded set as the native
  fallback, so web needs zero fetch plumbing.
- **Time and randomness** already go through engine-safe paths
  (`web_time`, the fixed-seed `Lcg`); keep new code off `std::time::Instant`.

## Toolchain quirks (worth knowing on Arch/CachyOS)

The build script pins two things down so they can't bite:

- **LTO is forced off for the wasm target only.** CachyOS bakes
  `target-cpu=x86-64-v3` into the `rust-wasm` std's bitcode; cross-crate
  LTO re-codegens it and spams `'x86-64-v3' is not a recognized
  processor` (harmless but deafening). Native builds keep the workspace
  thin-LTO profile.
- **wasm-bindgen CLI/lock version match** is enforced, because a mismatch
  produces a bundle that fails at runtime with opaque errors.

## Shipping to itch.io

1. `tools/build_web.sh`
2. Zip the **contents** of `dist-web/` (index.html at the zip root).
3. Upload to an itch HTML project, "this file will be played in the
   browser", and enable **SharedArrayBuffer support** if asked.

For native downloads, `tools/package_demo.sh` builds a Linux tarball
(and `--windows` a cross-built zip) with `assets/` laid out next to the
executable exactly as the binary expects.
