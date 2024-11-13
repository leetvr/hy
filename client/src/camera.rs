use ::dolly::prelude::{CameraRig, Position, Smooth, YawPitch};
use glam::{Quat, Vec3};

#[derive(Debug)]
pub struct FlyCamera {
    rig: CameraRig,
    pub movement_left: f32,
    pub movement_right: f32,
    pub movement_up: f32,
    pub movement_down: f32,
    pub movement_forward: f32,
    pub movement_backward: f32,
    pub boost: f32,
}

impl FlyCamera {
    pub fn new(
        initial_position: Vec3,
        initial_yaw_degrees: f32,
        initial_pitch_degrees: f32,
    ) -> FlyCamera {
        FlyCamera {
            rig: CameraRig::builder()
                .with(Position::new(initial_position))
                .with(
                    YawPitch::new()
                        .pitch_degrees(initial_pitch_degrees)
                        .yaw_degrees(initial_yaw_degrees),
                )
                .with(Smooth::new_position_rotation(1.0, 1.0))
                .build(),
            movement_left: 0.,
            movement_right: 0.,
            movement_up: 0.,
            movement_down: 0.,
            movement_forward: 0.,
            movement_backward: 0.,
            boost: 0.,
        }
    }

    pub(crate) fn update(&mut self, dt: f32) {
        // simple fly-cam impl
        let move_vec = Quat::from(self.rig.final_transform.rotation)
            * Vec3::new(
                self.movement_right - self.movement_left,
                self.movement_up - self.movement_down,
                self.movement_backward - self.movement_forward,
            )
            .normalize_or_zero()
            * 10.0f32.powf(self.boost);

        self.rig
            .driver_mut::<Position>()
            .translate(move_vec * dt * 10.);
        self.rig.update(dt);
    }

    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        self.rig
            .driver_mut::<YawPitch>()
            .rotate_yaw_pitch(yaw, pitch);
    }

    pub fn position_and_rotation(&self) -> (Vec3, Quat) {
        self.rig.final_transform.into_position_rotation()
    }

    pub fn set_position_and_rotation(&mut self, position: Vec3, rotation: YawPitch) {
        self.rig.driver_mut::<Position>().position = position.into();
        *self.rig.driver_mut::<YawPitch>() = rotation;
    }
}
