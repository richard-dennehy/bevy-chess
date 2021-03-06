use bevy::app::{EventReader, Plugin};
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;
use bevy::input::mouse::MouseMotion;

pub struct OrbitCameraPlugin;
impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(rotate_camera);
    }
}

#[derive(Component)]
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
    mut mouse_motion: EventReader<MouseMotion>,
    time: Res<Time>,
    mouse: Res<Input<MouseButton>>,
) {
    let (mut transform, mut camera) = cameras.single_mut();

    let rotation_speed = 1.0 * time.delta_seconds();
    let mouse_sensitivity = 0.33;
    let recentre_speed = rotation_speed * 2.0;

    let yaw_offset = if mouse.pressed(MouseButton::Right) {
        let x_movement: f32 = mouse_motion.iter().map(|motion| motion.delta.x).sum();
        camera.yaw_offset - ((x_movement * mouse_sensitivity) * rotation_speed)
    } else {
        #[allow(clippy::float_equality_without_abs)]
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
