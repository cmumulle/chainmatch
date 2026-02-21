use serde::{Deserialize, Serialize};

/// Ball physics parameters. All units are SI (meters, seconds, kg).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallPhysicsParams {
    pub gravity: f32,
    pub air_drag: f32,
    pub magnus_coefficient: f32,
    pub restitution: f32,
    pub ball_mass: f32,
    pub ball_radius: f32,
    pub max_speed: f32,
}

impl BallPhysicsParams {
    /// Default hard court physics parameters.
    pub fn hard_court() -> Self {
        Self {
            gravity: -9.81,
            air_drag: 0.005,
            magnus_coefficient: 0.0008,
            restitution: 0.75,
            ball_mass: 0.057,
            ball_radius: 0.033,
            max_speed: 70.0,
        }
    }
}

impl Default for BallPhysicsParams {
    fn default() -> Self {
        Self::hard_court()
    }
}

/// Court physical dimensions.
pub struct CourtDimensions {
    pub length: f32,
    pub width: f32,
    pub service_box_depth: f32,
    pub net_height_center: f32,
    pub net_height_posts: f32,
    pub baseline_runoff: f32,
    pub side_runoff: f32,
}

impl CourtDimensions {
    pub fn standard() -> Self {
        Self {
            length: 23.77,
            width: 8.23,
            service_box_depth: 6.40,
            net_height_center: 0.914,
            net_height_posts: 1.067,
            baseline_runoff: 6.0,
            side_runoff: 3.66,
        }
    }
}

impl Default for CourtDimensions {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ball_params_valid() {
        let params = BallPhysicsParams::hard_court();
        assert!(params.gravity < 0.0);
        assert!(params.air_drag > 0.0);
        assert!(params.magnus_coefficient > 0.0);
        assert!(params.restitution > 0.0 && params.restitution < 1.0);
        assert!(params.ball_mass > 0.0);
        assert!(params.ball_radius > 0.0);
        assert!(params.max_speed > 0.0);
    }

    #[test]
    fn test_court_dimensions() {
        let dims = CourtDimensions::standard();
        assert!((dims.length - 23.77).abs() < 0.01);
        assert!((dims.width - 8.23).abs() < 0.01);
    }

    #[test]
    fn test_net_height() {
        let dims = CourtDimensions::standard();
        assert!((dims.net_height_center - 0.914).abs() < 0.001);
        assert!((dims.net_height_posts - 1.067).abs() < 0.001);
    }
}
