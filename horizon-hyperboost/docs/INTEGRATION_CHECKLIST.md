# Renderer integration checklist

Do not wire a policy function to a guessed symbol. For the exact pinned Horizon source revision, identify and document:

- [ ] Chunk/section render submission function and its call frequency.
- [ ] Camera frustum representation and whether it is already computed per frame.
- [ ] Section bounds and chunk coordinate conversion.
- [ ] Reliable exposure/occlusion representation for underground sections.
- [ ] Special render layers (water, glass, leaves, cutout, emissive, block entities).
- [ ] Entity render queue and conservative occlusion evidence.
- [ ] GPU backend and whether an async depth pyramid / occlusion query already exists.
- [ ] Chunk mesh invalidation path after block edits and neighbour updates.
- [ ] Threading guarantees and ownership of renderer state.

## Safety invariants

1. Never remove a section solely because `max_y <= 51`.
2. Never hide caves, ravines, tunnels, openings, translucent geometry, or special renderers based on the deep-terrain rule.
3. Unknown visibility means render.
4. Avoid blocking GPU readbacks.
5. Cache visibility; invalidate on relevant chunk changes.
6. Keep the 16-chunk full-detail region untouched by deep-terrain culling.
7. Test chunk boundaries and negative world coordinates.
