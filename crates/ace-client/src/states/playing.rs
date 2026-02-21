use bevy::prelude::*;
use crate::resources::court::{self, CourtEntity};
use crate::systems::movement;

pub fn on_enter(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Entering Playing state");
    court::spawn_court(&mut commands, &mut meshes, &mut materials);
    movement::spawn_player(&mut commands, &mut meshes, &mut materials);
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
