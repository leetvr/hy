use std::ops::Mul;

#[derive(Debug, Clone, Default, Copy)]
pub struct Transform {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        position: glam::Vec3::ZERO,
        rotation: glam::Quat::IDENTITY,
        scale: glam::Vec3::ONE,
    };

    pub fn new<V: Into<glam::Vec3>>(position: V, rotation: glam::Quat) -> Self {
        Self {
            position: position.into(),
            rotation,
            scale: glam::Vec3::ONE,
        }
    }

    pub fn new_with_scale<V: Into<glam::Vec3>>(
        position: V,
        rotation: glam::Quat,
        scale: glam::Vec3,
    ) -> Self {
        Self {
            position: position.into(),
            rotation,
            scale,
        }
    }

    /// Return the forward direction from the reference frame of this transform
    pub fn forward(&self) -> glam::Vec3 {
        self.rotation.mul_vec3(glam::Vec3::NEG_Z)
    }

    /// Return the up direction from the reference frame of this transform
    pub fn up(&self) -> glam::Vec3 {
        self.rotation.mul_vec3(glam::Vec3::Y)
    }

    /// Return the right direction from the reference frame of this transform
    pub fn right(&self) -> glam::Vec3 {
        self.rotation.mul_vec3(glam::Vec3::X)
    }

    pub fn as_affine(&self) -> glam::Affine3A {
        glam::Affine3A::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

impl From<Transform> for glam::Affine3A {
    fn from(value: Transform) -> Self {
        value.as_affine()
    }
}

impl Mul for Transform {
    type Output = Transform;

    fn mul(self, rhs: Self) -> Self::Output {
        let a = self.as_affine();
        let b = rhs.as_affine();
        let (scale, rotation, position) = (a * b).to_scale_rotation_translation();
        Transform {
            position,
            rotation,
            scale,
        }
    }
}
