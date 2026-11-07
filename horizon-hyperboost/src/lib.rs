#![forbid(unsafe_code)]
//! Horizon HyperBoost's engine-independent optimisation core.
//!
//! IMPORTANT: these are policy functions, not renderer hooks. Horizon must
//! supply trustworthy chunk exposure, frustum, occlusion and frame-budget
//! information from its renderer before any geometry can safely be skipped.

/// Chunk-space position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    /// Squared Euclidean distance in chunks. Uses i64 to avoid i32 overflow.
    pub fn distance_squared(self, other: Self) -> u64 {
        let dx = i64::from(self.x) - i64::from(other.x);
        let dz = i64::from(self.z) - i64::from(other.z);
        let squared = (i128::from(dx) * i128::from(dx))
            + (i128::from(dz) * i128::from(dz));
        squared.min(i128::from(u64::MAX)) as u64
    }
}

/// The LOD policy to apply to a chunk, measured from the camera's chunk.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LodTier {
    Full,
    High,
    Medium,
    Low,
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Chunks inside this radius are never removed by HyperBoost's deep-terrain rule.
    pub full_detail_radius: u32,
    /// Deep sections at or below this Y may be omitted only when proven enclosed.
    pub deep_terrain_max_y: i32,
    /// The LOD thresholds are chunk distances.
    pub high_lod_until: u32,
    pub medium_lod_until: u32,
    /// Entity occlusion must be stable for this many consecutive observations.
    pub entity_occlusion_frames: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            full_detail_radius: 16,
            deep_terrain_max_y: 51,
            high_lod_until: 32,
            medium_lod_until: 64,
            entity_occlusion_frames: 3,
        }
    }
}

/// Evidence collected by Horizon for one render section.
///
/// `fully_enclosed` must mean the section has no exposed-to-air path or
/// visible opening relevant to the camera. Do not set it merely because
/// the section's blocks are individually non-air.
#[derive(Clone, Copy, Debug, Default)]
pub struct SectionEvidence {
    pub min_y: i32,
    pub max_y: i32,
    pub fully_enclosed: bool,
    pub has_open_cave_or_ravine: bool,
    pub intersects_camera_frustum: bool,
    pub contains_translucent_or_special_geometry: bool,
}

/// Result of evaluating a terrain section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerrainDecision {
    Keep,
    SkipOutsideFrustum,
    SkipDeepEnclosed,
    Simplify(LodTier),
}

/// Conservative terrain policy. It never deep-culls a section with known
/// cave/ravine openings, special/translucent geometry, or uncertain exposure.
pub fn terrain_decision(
    camera: ChunkPos,
    section_chunk: ChunkPos,
    evidence: SectionEvidence,
    config: Config,
) -> TerrainDecision {
    if !evidence.intersects_camera_frustum {
        return TerrainDecision::SkipOutsideFrustum;
    }

    let radius_sq = u64::from(config.full_detail_radius) * u64::from(config.full_detail_radius);
    let distance_sq = camera.distance_squared(section_chunk);

    if distance_sq > radius_sq
        && evidence.max_y <= config.deep_terrain_max_y
        && evidence.fully_enclosed
        && !evidence.has_open_cave_or_ravine
        && !evidence.contains_translucent_or_special_geometry
    {
        return TerrainDecision::SkipDeepEnclosed;
    }

    let distance = integer_sqrt(distance_sq);
    let tier = if distance <= config.full_detail_radius as u64 {
        LodTier::Full
    } else if distance <= config.high_lod_until as u64 {
        LodTier::High
    } else if distance <= config.medium_lod_until as u64 {
        LodTier::Medium
    } else {
        LodTier::Low
    };

    if tier == LodTier::Full {
        TerrainDecision::Keep
    } else {
        TerrainDecision::Simplify(tier)
    }
}

/// Conservative entity visibility inputs. Horizon's renderer must provide
/// frustum and occlusion evidence; this crate does not guess from entity
/// coordinates or hide entities based on distance alone.
#[derive(Clone, Copy, Debug, Default)]
pub struct EntityEvidence {
    pub intersects_frustum: bool,
    pub occluded_consecutive_frames: u8,
    pub is_translucent_or_special: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityDecision {
    Render,
    SkipOutsideFrustum,
    SkipOccluded,
}

pub fn entity_decision(evidence: EntityEvidence, config: Config) -> EntityDecision {
    if !evidence.intersects_frustum {
        return EntityDecision::SkipOutsideFrustum;
    }
    if !evidence.is_translucent_or_special
        && config.entity_occlusion_frames > 0
        && evidence.occluded_consecutive_frames >= config.entity_occlusion_frames
    {
        return EntityDecision::SkipOccluded;
    }
    EntityDecision::Render
}

/// Small integer square root; avoids floating point in the per-section policy.
fn integer_sqrt(n: u64) -> u64 {
    if n < 2 { return n; }
    let mut x = n;
    let mut y = x / 2 + 1;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Select the next section work item by squared distance and stable sequence.
/// This is intentionally a simple deterministic primitive; the game adapter
/// should feed it a bounded queue rather than sorting every loaded chunk/frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkItem {
    pub pos: ChunkPos,
    pub sequence: u64,
}

pub fn prioritize_work(camera: ChunkPos, a: WorkItem, b: WorkItem) -> core::cmp::Ordering {
    camera.distance_squared(a.pos)
        .cmp(&camera.distance_squared(b.pos))
        .then_with(|| a.sequence.cmp(&b.sequence))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visible_section(max_y: i32) -> SectionEvidence {
        SectionEvidence {
            min_y: max_y - 15,
            max_y,
            fully_enclosed: true,
            has_open_cave_or_ravine: false,
            intersects_camera_frustum: true,
            contains_translucent_or_special_geometry: false,
        }
    }

    #[test]
    fn deep_enclosed_terrain_is_skipped_only_beyond_sixteen_chunks() {
        let cfg = Config::default();
        let section = visible_section(48);
        assert_eq!(
            terrain_decision(ChunkPos { x: 0, z: 0 }, ChunkPos { x: 17, z: 0 }, section, cfg),
            TerrainDecision::SkipDeepEnclosed
        );
        assert_eq!(
            terrain_decision(ChunkPos { x: 0, z: 0 }, ChunkPos { x: 16, z: 0 }, section, cfg),
            TerrainDecision::Keep
        );
    }

    #[test]
    fn caves_and_special_geometry_are_never_deep_culled() {
        let cfg = Config::default();
        let mut cave = visible_section(40);
        cave.has_open_cave_or_ravine = true;
        assert_ne!(
            terrain_decision(ChunkPos { x: 0, z: 0 }, ChunkPos { x: 40, z: 0 }, cave, cfg),
            TerrainDecision::SkipDeepEnclosed
        );

        let mut special = visible_section(40);
        special.contains_translucent_or_special_geometry = true;
        assert_ne!(
            terrain_decision(ChunkPos { x: 0, z: 0 }, ChunkPos { x: 40, z: 0 }, special, cfg),
            TerrainDecision::SkipDeepEnclosed
        );
    }

    #[test]
    fn above_y_51_is_not_deep_culled() {
        let cfg = Config::default();
        assert_ne!(
            terrain_decision(
                ChunkPos { x: 0, z: 0 },
                ChunkPos { x: 40, z: 0 },
                visible_section(52),
                cfg
            ),
            TerrainDecision::SkipDeepEnclosed
        );
    }

    #[test]
    fn out_of_frustum_is_skipped_first() {
        let cfg = Config::default();
        let mut evidence = visible_section(40);
        evidence.intersects_camera_frustum = false;
        assert_eq!(
            terrain_decision(ChunkPos { x: 0, z: 0 }, ChunkPos { x: 40, z: 0 }, evidence, cfg),
            TerrainDecision::SkipOutsideFrustum
        );
    }

    #[test]
    fn entity_occlusion_requires_stable_frames_and_respects_special_entities() {
        let cfg = Config::default();
        let hidden = EntityEvidence {
            intersects_frustum: true,
            occluded_consecutive_frames: 3,
            is_translucent_or_special: false,
        };
        assert_eq!(entity_decision(hidden, cfg), EntityDecision::SkipOccluded);

        let special = EntityEvidence { is_translucent_or_special: true, ..hidden };
        assert_eq!(entity_decision(special, cfg), EntityDecision::Render);

        let uncertain = EntityEvidence { occluded_consecutive_frames: 2, ..hidden };
        assert_eq!(entity_decision(uncertain, cfg), EntityDecision::Render);
    }

    #[test]
    fn distance_and_priority_are_deterministic() {
        let origin = ChunkPos { x: 0, z: 0 };
        assert_eq!(origin.distance_squared(ChunkPos { x: 3, z: 4 }), 25);
        let near = WorkItem { pos: ChunkPos { x: 2, z: 0 }, sequence: 10 };
        let far = WorkItem { pos: ChunkPos { x: 5, z: 0 }, sequence: 1 };
        assert_eq!(prioritize_work(origin, near, far), core::cmp::Ordering::Less);
    }
}
