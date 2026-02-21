pub mod hero_select;
pub mod menu;
pub mod playing;
pub mod post_match;

use bevy::prelude::*;
use crate::systems::{aiming, ball_physics, hud, input, movement, shot};

/// Top-level game states.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Menu,
    HeroSelect,
    Playing,
    PostMatch,
}

/// Plugin that registers all game states and their systems.
pub struct StatesPlugin;

impl Plugin for StatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .add_systems(OnEnter(GameState::Menu), menu::on_enter)
            .add_systems(OnExit(GameState::Menu), menu::on_exit)
            .add_systems(OnEnter(GameState::HeroSelect), hero_select::on_enter)
            .add_systems(OnExit(GameState::HeroSelect), hero_select::on_exit)
            .add_systems(OnEnter(GameState::Playing), (playing::on_enter, aiming::spawn_aim_indicator, hud::spawn_hud))
            .add_systems(OnExit(GameState::Playing), playing::on_exit)
            .add_systems(OnEnter(GameState::PostMatch), post_match::on_enter)
            .add_systems(OnExit(GameState::PostMatch), post_match::on_exit)
            .add_systems(Update, debug_state_transition)
            .add_systems(
                Update,
                movement::player_movement_system.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                ball_physics::ball_physics_system.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                ball_physics::debug_ball_launch.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                ball_physics::debug_ball_events.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                aiming::update_aim_target.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (shot::shot_charge_system, shot::shot_execution_system)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (shot::smash_detection_system, input::shot_modifier_cycle_system, input::shot_type_system)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (hud::update_power_bar, hud::update_modifier_label, hud::update_shot_type_label)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_event::<shot::ShotCharged>()
            .init_resource::<shot::ShotChargeState>()
            .init_resource::<input::ActiveShotModifier>()
            .init_resource::<input::ActiveShotType>()
            .init_resource::<input::SmashAvailable>()
            .add_event::<ball_physics::NetFault>()
            .add_event::<ball_physics::OutOfBounds>()
            .add_event::<ball_physics::ValidBounce>()
            .init_resource::<ball_physics::DebugLaunchIndex>()
            .init_resource::<aiming::AimTarget>()
            .insert_resource(Time::<Fixed>::from_hz(120.0));
    }
}

/// Temporary debug system: press Enter to cycle through states.
fn debug_state_transition(
    keyboard: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        let next = match current_state.get() {
            GameState::Menu => GameState::HeroSelect,
            GameState::HeroSelect => GameState::Playing,
            GameState::Playing => GameState::PostMatch,
            GameState::PostMatch => GameState::Menu,
        };
        info!("State transition: {:?} → {:?}", current_state.get(), next);
        next_state.set(next);
    }
}
