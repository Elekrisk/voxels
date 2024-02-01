use std::f32::consts::FRAC_PI_2;

use bevy_ecs::{
    component::Component,
    system::{Query, Res, ResMut},
};
use cgmath::{InnerSpace, Point3, Rad, Vector3, Zero};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    camera::Camera,
    game::{
        block::{Block, BlockId, BlockMetadata},
        physics,
    },
    input::Input,
};

use super::{block::BlockRegistry, physics::Collider, world::World, DeltaTime, Position, Velocity};

#[derive(Clone, Component)]
pub struct PlayerController {
    speed: f32,
    sensitivity: f32,
    mine_cooldown: f32,
    place_cooldown: f32,
    place_block_id: BlockId,
    noclip: bool,
    fly_trigger_cooldown: f32,
}

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

impl PlayerController {
    pub fn new() -> Self {
        Self {
            speed: 4.0,
            sensitivity: 1.0,
            mine_cooldown: 0.0,
            place_cooldown: 0.0,
            place_block_id: BlockId(1),
            noclip: true,
            fly_trigger_cooldown: 0.0,
        }
    }
}

pub fn update_system(
    mut query: Query<(
        &mut PlayerController,
        &Position,
        &mut Velocity,
        &mut Collider,
    )>,
    dt: Res<DeltaTime>,
    input: Res<Input>,
    mut camera: ResMut<Camera>,
    mut world: ResMut<World>,
    block_registry: Res<BlockRegistry>,
) {
    for (mut pc, pos, mut vel, mut col) in &mut query {
        let mut v = Vector3::zero();

        fn as_f32(b: bool) -> f32 {
            if b {
                1.0
            } else {
                0.0
            }
        }

        let amount_forward = as_f32(input.is_pressed(KeyCode::KeyW));
        let amount_backward = as_f32(input.is_pressed(KeyCode::KeyS));
        let amount_left = as_f32(input.is_pressed(KeyCode::KeyA));
        let amount_right = as_f32(input.is_pressed(KeyCode::KeyD));

        let delta = input.mouse_delta();
        let rotate_horizontal = delta.x;
        let rotate_vertical = delta.y;

        let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        v += forward * (amount_forward - amount_backward);
        v += right * (amount_right - amount_left);

        camera.yaw += Rad(rotate_horizontal) * pc.sensitivity * 0.001;
        camera.pitch += Rad(-rotate_vertical) * pc.sensitivity * 0.001;

        if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
            camera.pitch = -Rad(SAFE_FRAC_PI_2);
        } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
            camera.pitch = Rad(SAFE_FRAC_PI_2);
        }

        if !v.is_zero() {
            v = v.normalize() * pc.speed;
        }

        camera.position = pos.0 + Vector3::unit_y() * 1.6;

        vel.0.x = v.x;
        vel.0.z = v.z;

        if input.is_just_pressed(KeyCode::Space) {
            if !pc.noclip {
                vel.0.y = physics::jump_height_to_vel(1.2);
            }

            if pc.fly_trigger_cooldown > 0.0 {
                pc.noclip = !pc.noclip;
                col.enabled = !pc.noclip;
                col.gravity = !pc.noclip;
            } else {
                pc.fly_trigger_cooldown = 0.25;
            }
        }

        if input.is_mouse_just_pressed(MouseButton::Left)
            || input.is_mouse_pressed(MouseButton::Left) && pc.mine_cooldown <= 0.0
        {
            if let Some(hitinfo) =
                world.raycast(camera.position, camera.forward(), 10000.0, &block_registry)
            {
                world.place_block(
                    Block {
                        id: BlockId(0),
                        metadata: BlockMetadata(0),
                    },
                    hitinfo.position,
                );
                pc.mine_cooldown = 0.2;
            }
        }

        if input.is_mouse_just_pressed(MouseButton::Right)
            || input.is_mouse_pressed(MouseButton::Right) && pc.place_cooldown <= 0.0
        {
            if let Some(hitinfo) =
                world.raycast(camera.position, camera.forward(), 10000.0, &block_registry)
            {
                world.place_block(
                    Block {
                        id: pc.place_block_id,
                        metadata: BlockMetadata(0),
                    },
                    hitinfo.position + hitinfo.normal.cast::<isize>().unwrap(),
                );
                pc.place_cooldown = 0.2;
            }
        }

        if input.is_just_pressed(KeyCode::Digit1) {
            pc.place_block_id.0 = 1;
        }
        if input.is_just_pressed(KeyCode::Digit2) {
            pc.place_block_id.0 = 2;
        }
        if input.is_just_pressed(KeyCode::Digit3) {
            pc.place_block_id.0 = 3;
        }

        pc.mine_cooldown -= dt.0;
        pc.place_cooldown -= dt.0;
        pc.fly_trigger_cooldown -= dt.0;

        if input.is_just_pressed(KeyCode::KeyC) {
            pc.noclip = !pc.noclip;
            col.enabled = !pc.noclip;
            col.gravity = !pc.noclip;
        }

        if pc.noclip {
            vel.0.y = 0.0;

            if input.is_pressed(KeyCode::Space) {
                vel.0.y += pc.speed;
            }
            if input.is_pressed(KeyCode::ShiftLeft) {
                vel.0.y -= pc.speed;
            }
        }
    }
}
