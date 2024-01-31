use std::{f32::consts::FRAC_PI_2, sync::atomic::AtomicUsize, time::Duration};

use cgmath::{InnerSpace, Point3, Rad, Vector2, Vector3, Zero};
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use super::Game;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(usize);

impl EntityId {
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self(id)
    }
}

pub struct Entity {
    pub id: EntityId,
    pub pos: Point3<f32>,
    pub vel: Vector3<f32>,
    pub collider: Collider,
    pub components: Vec<Box<dyn Component>>,
}

pub struct Collider {
    pub extents: Vector3<f32>,
}

pub trait Component {
    fn update(&mut self, entity_id: EntityId, dt: Duration, commands: &mut Commands) {}
    fn keyboard_input(&mut self, event: KeyEvent, commands: &mut Commands) {}
    fn mouse_input(&mut self, delta: Vector2<f32>, commands: &mut Commands) {}
}

pub struct Commands {
    commands: Vec<Box<dyn FnOnce(&mut Game)>>,
}

impl Commands {
    pub fn new() -> Self {
        Self {
            commands: vec![],
        }
    }

    pub fn execute(self, game: &mut Game) {
        for command in self.commands {
            (command)(game);
        }
    }

    pub fn add(&mut self, f: impl FnOnce(&mut Game) + 'static) {
        self.commands.push(Box::new(f));
    }
}

#[derive(Clone)]
pub struct PlayerController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    speed: f32,
    sensitivity: f32,
}

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

impl PlayerController {
    pub fn new() -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            speed: 4.0,
            sensitivity: 1.0,
        }
    }
}

impl Component for PlayerController {
    fn update(&mut self, entity_id: EntityId, dt: Duration, commands: &mut Commands) {
        let s = self.clone();
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        commands.add(move |game| {
            let dt = dt.as_secs_f32();

            let Some(e) = game.entities.iter_mut().find(|e| e.id == entity_id) else {
                return;
            };

            let mut vel = Vector3::zero();

            let camera = &mut game.camera;


            let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
            let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
            let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
            vel += forward * (s.amount_forward - s.amount_backward);
            vel += right * (s.amount_right - s.amount_left);

            camera.yaw += Rad(s.rotate_horizontal) * s.sensitivity * dt;
            camera.pitch += Rad(-s.rotate_vertical) * s.sensitivity * dt;

            if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
                camera.pitch = -Rad(SAFE_FRAC_PI_2);
            } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
                camera.pitch = Rad(SAFE_FRAC_PI_2);
            }

            vel *= s.speed;
            
            e.vel.x = vel.x;
            e.vel.z = vel.z;

            camera.position = e.pos + Vector3::unit_y() * 1.6;
        })
    }

    fn keyboard_input(&mut self, event: KeyEvent, commands: &mut Commands) {
        let KeyEvent {
            physical_key,
            state,
            ..
        } = event;
        let PhysicalKey::Code(key) = physical_key else {
            return;
        };

        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            KeyCode::KeyW => {
                self.amount_forward = amount;
            }
            KeyCode::KeyS => {
                self.amount_backward = amount;
            }
            KeyCode::KeyA => {
                self.amount_left = amount;
            }
            KeyCode::KeyD => {
                self.amount_right = amount;
            }
            KeyCode::Space => {
                self.amount_up = amount;
            }
            KeyCode::ShiftLeft => {
                self.amount_down = amount;
            }
            _ => {}
        }
    }

    fn mouse_input(&mut self, delta: Vector2<f32>, commands: &mut Commands) {
        self.rotate_horizontal = delta.x;
        self.rotate_vertical = delta.y;
    }
}
