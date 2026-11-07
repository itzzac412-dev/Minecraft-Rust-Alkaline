# Rust Alkaline

**Native Rust optimisation mod project for Horizon. Initial tuning target: Apple M2 MacBook Air.**

Rust Alkaline is built on Horizon's official `horizon-mixins` native-mod template. It contains a conservative optimisation policy core for deep-terrain culling, distance-based LOD decisions, and entity-occlusion decisions.

## Current status — integration scaffold

This repository is a real native-mod template with a `.hnmod` packaging workflow. The policy functions are implemented and tested as Rust code, but they are **not yet connected to Horizon's terrain renderer or entity render queue**. The example block-break hook is intentionally inert. Therefore this build does not yet increase FPS.

A renderer adapter must be written against the exact Horizon source revision used by the player's launcher. Do not claim performance gains until a controlled in-game benchmark verifies them.

## Optimisation policy

- Full detail within 16 chunks.
- Beyond 16 chunks, sections at or below Y=51 may be skipped only when renderer evidence confirms they are fully enclosed.
- Preserve known caves, ravines, translucent blocks and special geometry.
- Use High / Medium / Low LOD policy tiers at increasing distances.
- Skip entities only when outside the frustum or persistently occluded.
- Unknown visibility means render; safety wins over aggressive culling.

## Critical compatibility note

The supplied official template pins Horizon `ba6344a59cb9e755b730dcf02285fc0d33b97d3c` and horizon-mixins `d9c83d7096b7ea5b83678da2efdfd6161f74997e`.

The user's launcher screenshot showed a different Horizon build (`native-6475f62-d93dcab6bc82`, Horizon commit prefix `6475f62`, mixins prefix `d93dcab6bc82`). Those are not interchangeable. The exact full commits and source hashes must be verified before changing `compatibility.json` or producing a bundle for that launcher. This template pin is retained rather than inventing a full commit or source hash.

## Build and validate the standalone policy

Requirements: Rust 1.95.0, Python 3.12+, Git.

```sh
cargo fetch --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
python3 tools/test_template.py
```

## Stage and build against the pinned Horizon checkout

Clone the exact pinned Horizon and horizon-mixins revisions as described in the original template workflow. Then:

```sh
python3 tools/horizon.py stage --horizon ../horizon
python3 tools/horizon.py check --horizon ../horizon
python3 tools/horizon.py test --horizon ../horizon
python3 tools/horizon.py build --horizon ../horizon
python3 tools/package.py
```

The package script writes `target/rust_alkaline-0.1.0.hnmod`. The launcher must use the same Horizon and horizon-mixins pins. Do not install this pin into the user's different Performance build.

## Next renderer integration work

1. Obtain the full Horizon commit and mixins commit corresponding to the user's installed Performance build.
2. Verify the renderer source paths and hashes at that commit.
3. Identify section visibility, frustum culling, terrain mesh submission, entity render queue and cache invalidation APIs.
4. Add small typed hooks through `horizon-mixins`; never guess source targets.
5. Connect the policy functions to real renderer evidence and keep unknown cases visible.
6. Run native checks and benchmark the same world, camera, resolution and settings before/after.

## Upstream template

Based on the official Horizon native mod template supplied for this project. Horizon and horizon-mixins source are not included. No Minecraft assets or game binaries are included.

License: MIT.
