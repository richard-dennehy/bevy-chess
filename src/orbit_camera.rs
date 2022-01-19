use bevy::app::{AppBuilder, Plugin};
use bevy::prelude::{Input, IntoSystem, KeyCode, Mat3, Query, Res, Time, Transform, Vec3};
use std::f32::consts::FRAC_PI_2;

pub struct OrbitCameraPlugin;
impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(rotate_camera.system());
    }
}

pub struct GameCamera {
    eye: Vec3,
    target: Vec3,
    pitch: f32,
    initial_yaw: f32,
    yaw_offset: f32,
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

        GameCamera {
            eye,
            target,
            pitch,
            initial_yaw: yaw,
            yaw_offset: 0.0,
        }
    }
}

fn rotate_camera(
    mut cameras: Query<(&mut Transform, &mut GameCamera)>,
    time: Res<Time>,
    keyboard: Res<Input<KeyCode>>,
) {
    let (mut transform, mut camera) = cameras.single_mut().expect("no primary camera");

    let rotation_speed = 1.0 * time.delta_seconds();
    let recentre_speed = rotation_speed * 2.5;

    let yaw_offset = if keyboard.pressed(KeyCode::Left) {
        camera.yaw_offset - rotation_speed
    } else if keyboard.pressed(KeyCode::Right) {
        camera.yaw_offset + rotation_speed
    } else {
        if (camera.yaw_offset.abs() - recentre_speed) < f32::EPSILON {
            0.0
        } else if camera.yaw_offset < 0.0 {
            camera.yaw_offset + recentre_speed
        } else {
            camera.yaw_offset - recentre_speed
        }
    };

    if yaw_offset.abs() > FRAC_PI_2 {
        return;
    }

    let rotated_look_dir = {
        let ray = Mat3::from_rotation_y(camera.initial_yaw + yaw_offset) * Vec3::Z;
        let pitch_axis = ray.cross(Vec3::Y);

        Mat3::from_axis_angle(pitch_axis, camera.pitch) * ray
    };
    let look_dir_magnitude = (camera.eye - camera.target).length();
    camera.eye = camera.target + (rotated_look_dir * look_dir_magnitude);
    camera.yaw_offset = yaw_offset;

    *transform = Transform::from_translation(camera.eye).looking_at(camera.target, Vec3::Y);
}
