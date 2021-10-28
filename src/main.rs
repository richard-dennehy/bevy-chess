use bevy::input::system::exit_on_esc_system;
use crate::board::BoardPlugin;
use crate::pieces::PiecePlugin;
use crate::ui::UiPlugin;
use bevy::prelude::*;
use bevy_mod_picking::{PickingCameraBundle, PickingPlugin};

mod board;
mod pieces;
mod ui;

#[cfg(test)]
mod tests;

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            width: 1600.0,
            height: 800.0,
            title: "CHESS".into(),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(PickingPlugin)
        .add_plugin(BoardPlugin)
        .add_plugin(PiecePlugin)
        .add_plugin(UiPlugin)
        .add_startup_system(setup.system())
        .add_system(exit_on_esc_system.system())
        .run();
}

fn setup(mut commands: Commands) {
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_matrix(Mat4::from_rotation_translation(
                Quat::from_xyzw(-0.3, -0.5, -0.3, 0.5).normalize(),
                Vec3::new(-4.0, 15.0, 4.0),
            )),
            ..Default::default()
        })
        .insert_bundle(PickingCameraBundle::default());

    commands.spawn_bundle(LightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });
}
