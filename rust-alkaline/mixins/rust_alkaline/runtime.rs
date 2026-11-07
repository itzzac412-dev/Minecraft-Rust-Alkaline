// Rust Alkaline — conservative renderer optimisation policy core.
//
// This module is injected into Horizon's game crate by horizon-mixins.
// The functions below are safe policy helpers; they do not directly issue GPU
// commands. Renderer integration must supply trustworthy visibility evidence.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn distance_squared(self, other: Self) -> u64 {
        let dx = i128::from(self.x) - i128::from(other.x);
        let dz = i128::from(self.z) - i128::from(other.z);
        (dx * dx + dz * dz).min(i128::from(u64::MAX)) as u64
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub full_detail_radius: u32,
    pub deep_terrain_max_y: i32,
    pub high_lod_until: u32,
    pub medium_lod_until: u32,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LodTier { Full, High, Medium, Low }

#[derive(Clone, Copy, Debug, Default)]
pub struct SectionEvidence {
    pub max_y: i32,
    pub fully_enclosed: bool,
    pub has_open_cave_or_ravine: bool,
    pub intersects_camera_frustum: bool,
    pub contains_translucent_or_special_geometry: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerrainDecision {
    Keep,
    SkipOutsideFrustum,
    SkipDeepEnclosed,
    Simplify(LodTier),
}

pub fn terrain_decision(
    camera: ChunkPos,
    section_chunk: ChunkPos,
    evidence: SectionEvidence,
    config: Config,
) -> TerrainDecision {
    if !evidence.intersects_camera_frustum {
        return TerrainDecision::SkipOutsideFrustum;
    }

    let radius_sq = u64::from(config.full_detail_radius)
        * u64::from(config.full_detail_radius);
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

    if tier == LodTier::Full { TerrainDecision::Keep }
    else { TerrainDecision::Simplify(tier) }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EntityEvidence {
    pub intersects_frustum: bool,
    pub occluded_consecutive_frames: u8,
    pub is_translucent_or_special: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityDecision { Render, SkipOutsideFrustum, SkipOccluded }

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

// The template's example hook is intentionally inert until a real renderer
// hook is verified for the user's exact Horizon commit.
pub fn before_break(_pos: [i32; 3]) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn enclosed(max_y: i32) -> SectionEvidence {
        SectionEvidence {
            max_y,
            fully_enclosed: true,
            intersects_camera_frustum: true,
            ..SectionEvidence::default()
        }
    }

    #[test]
    fn deep_enclosed_sections_only_cull_beyond_sixteen_chunks() {
        let cfg = Config::default();
        assert_eq!(
            terrain_decision(ChunkPos { x: 0, z: 0 }, ChunkPos { x: 17, z: 0 }, enclosed(48), cfg),
            TerrainDecision::SkipDeepEnclosed
        );
        assert_eq!(
            terrain_decision(ChunkPos { x: 0, z: 0 }, ChunkPos { x: 16, z: 0 }, enclosed(48), cfg),
            TerrainDecision::Keep
        );
    }

    #[test]
    fn caves_and_special_geometry_are_preserved() {
        let cfg = Config::default();
        let cave = SectionEvidence { has_open_cave_or_ravine: true, ..enclosed(48) };
        assert_ne!(
            terrain_decision(ChunkPos { x: 0, z: 0 }, ChunkPos { x: 30, z: 0 }, cave, cfg),
            TerrainDecision::SkipDeepEnclosed
        );
        let special = SectionEvidence {
            contains_translucent_or_special_geometry: true,
            ..enclosed(48)
        };
        assert_ne!(
            terrain_decision(ChunkPos { x: 0, z: 0 }, ChunkPos { x: 30, z: 0 }, special, cfg),
            TerrainDecision::SkipDeepEnclosed
        );
    }

    #[test]
    fn y_above_51_is_not_deep_culled() {
        assert_ne!(
            terrain_decision(
                ChunkPos { x: 0, z: 0 },
                ChunkPos { x: 0, z: 30 },
                enclosed(52),
                Config::default()
            ),
            TerrainDecision::SkipDeepEnclosed
        );
    }

    #[test]
    fn entity_occlusion_requires_three_frames_and_preserves_special_entities() {
        let cfg = Config::default();
        let hidden = EntityEvidence {
            intersects_frustum: true,
            occluded_consecutive_frames: 3,
            is_translucent_or_special: false,
        };
        assert_eq!(entity_decision(hidden, cfg), EntityDecision::SkipOccluded);
        assert_eq!(
            entity_decision(EntityEvidence { is_translucent_or_special: true, ..hidden }, cfg),
            EntityDecision::Render
        );
    }
}
