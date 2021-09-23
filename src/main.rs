use bevy::prelude::*;
use bevy_mod_picking::{PickingPlugin, PickingCameraBundle};
use crate::board::BoardPlugin;

mod board;
mod pieces;

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
        .add_startup_system(setup.system())
        .add_startup_system(pieces::create_pieces.system())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_matrix(Mat4::from_rotation_translation(
            Quat::from_xyzw(-0.3, -0.5, -0.3, 0.5).normalize(),
            Vec3::new(-4.0, 15.0, 4.0),
        )),
        ..Default::default()
    }).insert_bundle(PickingCameraBundle::default());

    commands.spawn_bundle(LightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });
}
