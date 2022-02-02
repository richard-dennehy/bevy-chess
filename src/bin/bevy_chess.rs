extern crate bevy_chess;

use bevy::input::system::exit_on_esc_system;
use bevy::prelude::*;
use bevy_chess::ui::UiPlugin;
use bevy_mod_picking::{PickingCameraBundle, PickingPlugin};
use bevy_chess::systems::chess::ChessPlugin;
use bevy_chess::systems::orbit_camera::{GameCamera, OrbitCameraPlugin};

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            width: 1600.0,
            height: 800.0,
            title: "CHESS".into(),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(PickingPlugin)
        .add_plugin(ChessPlugin)
        .add_plugin(OrbitCameraPlugin)
        .add_plugin(UiPlugin)
        .add_startup_system(setup.system())
        .add_system(exit_on_esc_system.system())
        .run();
}

fn setup(mut commands: Commands) {
    commands
        .spawn_bundle(PerspectiveCameraBundle::default())
        .insert_bundle(PickingCameraBundle::default())
        .insert(GameCamera::new(Vec3::new(0.0, 13.0, -9.0), Vec3::ZERO));

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(1.0, 10.0, 2.0),
        point_light: PointLight {
            intensity: 400.0,
            range: 60.0,
            ..Default::default()
        },
        ..Default::default()
    });
}
