use serde::{Deserialize, Serialize};

/// Newtype wrapper for hero identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeroId(pub u8);

/// Hero archetype classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Archetype {
    BaselineBrawler,
    ServeAndVolley,
    CounterPuncher,
    AllRounder,
}

/// Full hero stat block. All stats are 0.0–1.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroStats {
    pub id: HeroId,
    pub name: String,
    pub archetype: Archetype,
    pub serve_power: f32,
    pub serve_accuracy: f32,
    pub forehand_power: f32,
    pub backhand_power: f32,
    pub volley_skill: f32,
    pub spin_control: f32,
    pub speed: f32,
    pub acceleration: f32,
    pub stamina: f32,
    pub reach: f32,
}

/// Type of shot being executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShotType {
    Flat,
    Topspin,
    Slice,
    Lob,
    DropShot,
    Smash,
}

/// Shot spin modifier (applied on top of shot type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShotModifier {
    Flat,
    Topspin,
    Slice,
}

/// Court surface with associated physics parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CourtSurface {
    Hard,
    Clay,
    Grass,
}

impl CourtSurface {
    /// Ball restitution coefficient for this surface.
    pub fn restitution(&self) -> f32 {
        match self {
            CourtSurface::Hard => 0.75,
            CourtSurface::Clay => 0.70,
            CourtSurface::Grass => 0.68,
        }
    }

    /// Friction coefficient for this surface.
    pub fn friction(&self) -> f32 {
        match self {
            CourtSurface::Hard => 0.60,
            CourtSurface::Clay => 0.80,
            CourtSurface::Grass => 0.45,
        }
    }

    /// Speed multiplier (lower = slower court).
    pub fn speed_factor(&self) -> f32 {
        match self {
            CourtSurface::Hard => 1.0,
            CourtSurface::Clay => 0.85,
            CourtSurface::Grass => 1.10,
        }
    }
}

/// Match type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchType {
    Friendly,
    Ranked,
    Tournament,
}

/// Player identity as a 32-byte hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub [u8; 32]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hero_stats_serialize_roundtrip() {
        let stats = HeroStats {
            id: HeroId(0),
            name: "Viktor".to_string(),
            archetype: Archetype::BaselineBrawler,
            serve_power: 0.85,
            serve_accuracy: 0.65,
            forehand_power: 0.90,
            backhand_power: 0.75,
            volley_skill: 0.45,
            spin_control: 0.70,
            speed: 0.60,
            acceleration: 0.55,
            stamina: 0.80,
            reach: 0.75,
        };

        let bytes = bincode::serialize(&stats).unwrap();
        let deserialized: HeroStats = bincode::deserialize(&bytes).unwrap();

        assert_eq!(deserialized.id, stats.id);
        assert_eq!(deserialized.name, stats.name);
        assert_eq!(deserialized.archetype, stats.archetype);
        assert_eq!(deserialized.serve_power, stats.serve_power);
        assert_eq!(deserialized.forehand_power, stats.forehand_power);
        assert_eq!(deserialized.speed, stats.speed);
    }

    #[test]
    fn test_shot_type_variants() {
        let variants = [
            ShotType::Flat,
            ShotType::Topspin,
            ShotType::Slice,
            ShotType::Lob,
            ShotType::DropShot,
            ShotType::Smash,
        ];

        for shot in &variants {
            let bytes = bincode::serialize(shot).unwrap();
            let deserialized: ShotType = bincode::deserialize(&bytes).unwrap();
            assert_eq!(*shot, deserialized);
        }
    }

    #[test]
    fn test_shot_modifier_roundtrip() {
        let modifiers = [ShotModifier::Flat, ShotModifier::Topspin, ShotModifier::Slice];

        for modifier in &modifiers {
            let bytes = bincode::serialize(modifier).unwrap();
            let deserialized: ShotModifier = bincode::deserialize(&bytes).unwrap();
            assert_eq!(*modifier, deserialized);
        }
    }

    #[test]
    fn test_court_surface_params() {
        // Each surface should have distinct physics params
        assert_ne!(CourtSurface::Hard.restitution(), CourtSurface::Clay.restitution());
        assert_ne!(CourtSurface::Clay.restitution(), CourtSurface::Grass.restitution());
        assert_ne!(CourtSurface::Hard.friction(), CourtSurface::Grass.friction());
        assert_ne!(CourtSurface::Hard.speed_factor(), CourtSurface::Clay.speed_factor());

        // Roundtrip
        for surface in &[CourtSurface::Hard, CourtSurface::Clay, CourtSurface::Grass] {
            let bytes = bincode::serialize(surface).unwrap();
            let deserialized: CourtSurface = bincode::deserialize(&bytes).unwrap();
            assert_eq!(*surface, deserialized);
        }
    }

    #[test]
    fn test_match_type_roundtrip() {
        let types = [MatchType::Friendly, MatchType::Ranked, MatchType::Tournament];

        for mt in &types {
            let bytes = bincode::serialize(mt).unwrap();
            let deserialized: MatchType = bincode::deserialize(&bytes).unwrap();
            assert_eq!(*mt, deserialized);
        }
    }

    #[test]
    fn test_player_id_roundtrip() {
        let id = PlayerId([42u8; 32]);
        let bytes = bincode::serialize(&id).unwrap();
        let deserialized: PlayerId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_hero_id_roundtrip() {
        let id = HeroId(7);
        let bytes = bincode::serialize(&id).unwrap();
        let deserialized: HeroId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_archetype_roundtrip() {
        let archetypes = [
            Archetype::BaselineBrawler,
            Archetype::ServeAndVolley,
            Archetype::CounterPuncher,
            Archetype::AllRounder,
        ];

        for arch in &archetypes {
            let bytes = bincode::serialize(arch).unwrap();
            let deserialized: Archetype = bincode::deserialize(&bytes).unwrap();
            assert_eq!(*arch, deserialized);
        }
    }
}
