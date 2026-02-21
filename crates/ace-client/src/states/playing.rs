use bevy::prelude::*;
use crate::resources::court::{self, CourtEntity};
use crate::systems::{ball_physics, movement};

pub fn on_enter(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Entering Playing state");
    court::spawn_court(&mut commands, &mut meshes, &mut materials);
    movement::spawn_player(&mut commands, &mut meshes, &mut materials);
    // Drop ball from 3m height for bounce test
    ball_physics::spawn_ball(&mut commands, &mut meshes, &mut materials, Vec3::new(0.0, 3.0, 0.0));
}

pub fn on_exit(
    mut commands: Commands,
    query: Query<Entity, With<CourtEntity>>,
) {
    info!("Exiting Playing state");
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
