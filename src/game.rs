use std::{io::Write, sync::Arc, time::Duration};

use bevy_ecs::{
    component::Component, schedule::{Schedule, ScheduleLabel}, system::Resource
};
use cgmath::{EuclideanSpace, InnerSpace, Point3, Quaternion, Rotation3, Vector2, Vector3};
use wgpu::RenderPass;
use winit::event::{ElementState, KeyEvent, MouseButton};

use crate::{
    assets::AssetManager,
    camera::Camera,
    input::Input,
    mesh::{DrawModel, Mesh},
    meshifier::ChunkMeshifier,
    object::Object,
    Instance,
};

use self::{
    atlas::Atlas, block::{BlockAttributes, BlockId, BlockRegistry}, physics::Collider, player::PlayerController, world::World
};

pub mod atlas;
pub mod block;
pub mod chunk;
mod player;
pub mod world;
mod physics;

#[derive(Clone, Copy, PartialEq, Component)]
pub struct Position(pub Point3<f32>);
#[derive(Clone, Copy, PartialEq, Component)]
pub struct Velocity(pub Vector3<f32>);

pub struct Game {
    atlas: Atlas,
    chunk_meshifier: ChunkMeshifier,
    ecs_world: bevy_ecs::world::World,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ScheduleLabel)]
pub enum ScheduleStage {
    Update,
}

#[derive(Clone, Copy, PartialEq, Resource)]
pub struct DeltaTime(pub f32);

impl Game {
    pub fn new(asset_manager: &mut AssetManager) -> Self {
        let material = asset_manager.load_material("assets/atlas.png").unwrap();
        let atlas = Atlas::new(material, 16);

        let mut block_registry = BlockRegistry::new();

        let air_block_attr = BlockAttributes {
            transparent: true,
            invisible: true,
            uv_coords: [0, 0].into(),
        };
        block_registry.register(BlockId(0), air_block_attr);

        let dirt_block_attr = BlockAttributes {
            transparent: false,
            invisible: false,
            uv_coords: [0, 0].into(),
        };
        block_registry.register(BlockId(1), dirt_block_attr);

        let stone_block_attr = BlockAttributes {
            transparent: false,
            invisible: false,
            uv_coords: [1, 0].into(),
        };
        block_registry.register(BlockId(2), stone_block_attr);

        let neco_arg_block_attr = BlockAttributes {
            transparent: false,
            invisible: false,
            uv_coords: [2, 0].into(),
        };
        block_registry.register(BlockId(3), neco_arg_block_attr);

        let mut world = World::new();

        for x in 0..=0 {
            for z in 0..=0 {
                for y in 0..=0 {
                    world.generate_chunk([x, y, z]);
                }
            }
        }

        let camera = Camera::new([0.0, 0.0, 0.0], cgmath::Deg(0.0), cgmath::Deg(0.0));

        world.raycast([8.5, -1.0, 8.5].into(), [0.0, 1.0, 0.0].into(), 100.0, &block_registry);

        let input = Input::new();

        let mut ecs_world = bevy_ecs::world::World::new();
        ecs_world.insert_resource(world);
        ecs_world.insert_resource(block_registry);
        ecs_world.insert_resource(camera);
        ecs_world.insert_resource(input);
        ecs_world.insert_resource(DeltaTime(1.0 / 60.0));

        let mut schedule = Schedule::new(ScheduleStage::Update);
        schedule.add_systems(player::update_system);
        schedule.add_systems(physics::physics_system);
        ecs_world.add_schedule(schedule);
        
        ecs_world.spawn((
            Position([8.0, 20.0, 8.0].into()),
            Velocity([0.0, 0.0, 0.0].into()),
            Collider {
                enabled: false,
                gravity: false,
                extents: [0.5, 1.8, 0.5].into()
            },
            PlayerController::new(),
        ));

        Self {
            atlas,
            chunk_meshifier: ChunkMeshifier::new(),
            ecs_world,
        }
    }

    pub fn update(&mut self, dt: Duration) {
        std::io::stdout().flush().unwrap();
        self.ecs_world.resource_mut::<DeltaTime>().0 = dt.as_secs_f32();
        self.ecs_world.run_schedule(ScheduleStage::Update);
        self.ecs_world.resource_mut::<Input>().end_frame();
    }

    pub fn keyboard_input(&mut self, event: KeyEvent) {
        self.ecs_world
            .resource_mut::<Input>()
            .process_key_event(event);
    }

    pub fn mouse_input(&mut self, delta: Vector2<f32>) {
        std::io::stdout().flush().unwrap();
        self.ecs_world
            .resource_mut::<Input>()
            .process_mouse_move(delta);
    }

    pub fn mouse_button_input(&mut self, button: MouseButton, state: ElementState) {
        self.ecs_world.resource_mut::<Input>().process_mouse_input(button, state);
    }

    pub fn camera(&self) -> &Camera {
        self.ecs_world.resource::<Camera>()
    }

    pub fn get_objects_to_render(&mut self, device: &wgpu::Device) -> Vec<Object> {
        let world = self.ecs_world.resource::<World>();
        let block_registry = self.ecs_world.resource::<BlockRegistry>();
        world
            .chunks
            .iter()
            .map(|chunk| {
                let mesh =
                    self.chunk_meshifier
                        .meshify(world, chunk, &self.atlas, block_registry, device);
                let instance = Instance {
                    position: chunk.pos.cast::<f32>().unwrap() * 16.0,
                    rotation: Quaternion::from_angle_z(cgmath::Deg(0.0)),
                };

                Object::new(mesh, instance, device)
            })
            .collect()
    }
}
