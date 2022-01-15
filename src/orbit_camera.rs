use std::f32::consts::FRAC_PI_2;
use bevy::app::{AppBuilder, Plugin};
use bevy::prelude::{Input, IntoSystem, KeyCode, Mat3, Query, Res, Transform, Vec3};

pub struct OrbitCameraPlugin;
impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(move_camera.system());
    }
}

pub struct GameCamera {
    eye: Vec3,
    target: Vec3,
    pitch: f32,
    initial_yaw: f32,
}

impl GameCamera {
    pub fn new(eye: Vec3, target: Vec3) -> Self {
        let look_dir = (eye - target).normalize();
        let look_dir_xz = Vec3::new(look_dir.x, 0.0, look_dir.z);

        let yaw = if look_dir.x > 0.0 {
            look_dir_xz.angle_between(Vec3::Z)
        } else {
            -look_dir_xz.angle_between(Vec3::Z)
        };

        let pitch = if look_dir.y > 0.0 {
            look_dir_xz.angle_between(look_dir)
        } else {
            -look_dir_xz.angle_between(look_dir)
        };

        GameCamera { eye, target, pitch, initial_yaw: yaw }
    }
}

fn move_camera(
    mut cameras: Query<(&mut Transform, &mut GameCamera)>,
    keyboard: Res<Input<KeyCode>>,
) {
    let (mut transform, mut camera) = cameras.single_mut().expect("no primary camera");

    let look_dir = (camera.eye - camera.target).normalize();
    let look_dir_xz = Vec3::new(look_dir.x, 0.0, look_dir.z);

    let mut yaw = if look_dir.x > 0.0 {
        look_dir_xz.angle_between(Vec3::Z)
    } else {
        -look_dir_xz.angle_between(Vec3::Z)
    };

    let increment = 0.01;
    if keyboard.pressed(KeyCode::Left) {
        yaw -= increment;
    };

    if keyboard.pressed(KeyCode::Right) {
        yaw += increment;
    };

    let delta = (camera.initial_yaw.abs() - yaw.abs()).abs();
    if delta > FRAC_PI_2 {
        return;
    }

    let rotated_look_dir = {
        let ray = Mat3::from_rotation_y(yaw) * Vec3::Z;
        let pitch_axis = ray.cross(Vec3::Y);

        Mat3::from_axis_angle(pitch_axis, camera.pitch) * ray
    };
    let look_dir_magnitude = (camera.eye - camera.target).length();
    camera.eye = camera.target + (rotated_look_dir * look_dir_magnitude);

    *transform = Transform::from_translation(camera.eye).looking_at(camera.target, Vec3::Y);
}
