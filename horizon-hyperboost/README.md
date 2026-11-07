# Horizon HyperBoost

**A native-Rust optimisation core for Horizon on macOS (initial target: Apple M2 MacBook Air).**

HyperBoost is designed around conservative terrain visibility, distant LOD policy, entity visibility evidence, and bounded chunk-work scheduling.

> **Status: engine-independent core / integration prototype. This is not yet an installable `.hnmod` and does not change FPS in Horizon by itself.** The renderer hook adapter is deliberately not fabricated: Horizon native mods patch exact Rust functions on an exact Horizon commit. The current public native-mod template pins a specific Nightly, and the current user has not supplied the exact commit of their launcher-installed Stable build. A real mod must bind these policies to verified renderer hooks and be tested against that exact source revision.

## Goals

- Preserve full-detail rendering within 16 chunks.
- Beyond 16 chunks, allow fully enclosed terrain sections at or below Y=51 to be omitted.
- Never deep-cull sections known to contain cave/ravine openings or translucent/special geometry.
- Use conservative LOD tiers beyond the full-detail radius.
- Skip entities only when the renderer proves they are outside the camera frustum or persistently occluded.
- Prioritise nearby chunk work deterministically.

## Why the Y=51 rule is evidence-based

A simple `y <= 51` test is unsafe: it can remove visible cave walls, ravines, exposed cliff faces, mineshafts, and other openings. Horizon must supply a reliable `fully_enclosed` result from chunk/section visibility data. The core only skips deep terrain when all safety conditions pass.

Entity occlusion likewise requires renderer evidence (ideally a hierarchical-Z/depth-pyramid or conservative occlusion query). This library does not pretend it can infer visibility from entity distance alone.

## Build and test the core

Requires stable Rust and Cargo.

```sh
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
```

GitHub Actions runs the same checks on macOS and Linux.

## Native Horizon integration — next required step

Horizon native mods are `.hnmod` source-mixin bundles, not ordinary Rust dynamic libraries or Fabric JARs. The launcher compiles the mod into a custom game build. Each bundle must pin the exact Horizon and `horizon-mixins` revisions used by its hooks.

Before producing an installable bundle:

1. Record the exact Horizon version and commit used by the launcher's selected Stable instance.
2. Obtain the matching Horizon source and `horizon-mixins` revision.
3. Inspect the renderer's chunk-section visibility, mesh generation, entity render queue, and frame scheduling code.
4. Add narrow typed hooks at verified functions. Do not overwrite the renderer wholesale.
5. Feed real frustum, section exposure, cave/opening, translucent-geometry, and occlusion evidence into `terrain_decision` / `entity_decision`.
6. Run the official template's stage, check, test, and build process against the pinned source.
7. Validate in a world with caves, ravines, water, leaves, glass, entities, and chunk edits.

The official template and current pinning/build instructions are linked below. The public template currently documents a Nightly pin; do not silently substitute it for the user's Stable build.

## M2 MacBook Air tuning notes

- The M2 GPU is unified-memory and the Air is fanless. Sustained results depend on ambient temperature, resolution, memory pressure, and power mode.
- Start with conservative occlusion and LOD. Measure frame time and GPU/CPU time separately.
- Avoid synchronous GPU readbacks for occlusion; they can cost more than the geometry they save.
- Cache section visibility and invalidate it only when neighbouring chunk data changes.
- Never promise a fixed FPS uplift without measuring the same world, camera, resolution, and render settings before/after.

## Repository links

- Horizon native mod documentation: https://github.com/horizon-rust/horizon/blob/main/docs/mods.md
- Official native mod template: https://github.com/horizon-rust/horizon-native-mod-template
- Distant Horizons for Horizon (reference implementation; separate project): https://github.com/denizsahin7744/distant-horizons-for-horizon-client

## License

MIT. This repository contains no Minecraft assets, game binaries, or Horizon source code.
