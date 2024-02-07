use bevy_ecs::system::Resource;
use cgmath::*;
use std::f32;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use winit::keyboard::KeyCode;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug, Resource)]
pub struct Camera {
    pub position: Point3<f32>,
    pub yaw: Rad<f32>,
    pub pitch: Rad<f32>,
}

impl Camera {
    pub fn new(
        position: impl Into<Point3<f32>>,
        yaw: impl Into<Rad<f32>>,
        pitch: impl Into<Rad<f32>>,
    ) -> Self {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        Matrix4::look_to_rh(
            self.position,
            Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Vector3::unit_y(),
        )
    }

    pub fn forward(&self) -> Vector3<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();
        Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize()
    }

    pub fn frustum(&self, proj: &Projection) -> Frustum {
        let hnear = 2.0 * (proj.fovy / 2.0).tan() * proj.znear;
        let wnear = hnear * proj.aspect;

        let d = self.forward();
        let right = d.cross(Vector3::unit_y()).normalize();
        let up = right.cross(d);

        let fc = self.position + d * proj.zfar;

        let nc = self.position + d * proj.znear;

        let near = Plane {
            normal: d,
            point: nc,
        };
        let far = Plane {
            normal: -d,
            point: fc,
        };

        let aux = ((nc + up * hnear) - self.position).normalize();
        let top = Plane {
            normal: aux.cross(right),
            point: nc + up * hnear,
        };
        let aux = ((nc - up * hnear) - self.position).normalize();
        let bottom = Plane {
            normal: right.cross(aux),
            point: nc - up * hnear,
        };
        let aux = ((nc - right * wnear) - self.position).normalize();
        let left = Plane {
            normal: aux.cross(up),
            point: nc - right * hnear,
        };
        let aux = ((nc + right * hnear) - self.position).normalize();
        let right = Plane {
            normal: up.cross(aux),
            point: nc + right * hnear,
        };

        Frustum::new(
            near,
            far,
            left,
            right,
            top,
            bottom,
        )
    }
}

pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new(width: u32, height: u32, fovy: impl Into<Rad<f32>>, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    pub fn new(
        near: Plane,
        far: Plane,
        left: Plane,
        right: Plane,
        top: Plane,
        bottom: Plane,
    ) -> Self {
        Self {
            planes: [near, far, left, right, top, bottom],
        }
    }

    pub fn contains_point(&self, point: Point3<f32>) -> bool {
        for plane in self.planes {
            if plane.sdf(point) < 0.0 {
                return false;
            }
        }

        true
    }

    pub fn contains_sphere(&self, sphere: Sphere) -> bool {
        for plane in self.planes {
            if plane.sdf(sphere.center) < -sphere.radius {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Plane {
    normal: Vector3<f32>,
    point: Point3<f32>,
}

impl Plane {
    pub fn sdf(&self, point: Point3<f32>) -> f32 {
        self.normal.dot(point - self.point)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Sphere {
    pub center: Point3<f32>,
    pub radius: f32,
}
