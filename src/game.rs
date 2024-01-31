use std::{sync::Arc, time::Duration};

use cgmath::{EuclideanSpace, Quaternion, Rotation3, Vector2, Vector3};
use wgpu::RenderPass;
use winit::event::KeyEvent;

use crate::{
    assets::AssetManager,
    camera::Camera,
    mesh::{DrawModel, Mesh},
    meshifier::ChunkMeshifier,
    object::Object,
    Instance,
};

use self::{
    atlas::Atlas, block::{BlockAttributes, BlockId, BlockRegistry}, entity::{Collider, Commands, Entity, EntityId, PlayerController}, physics_engine::PhysicsEngine, player::Player, world::World
};

pub mod atlas;
pub mod block;
pub mod chunk;
mod entity;
mod physics_engine;
mod player;
pub mod world;

pub struct Game {
    pub world: World,
    pub block_registry: BlockRegistry,
    atlas: Atlas,
    chunk_meshifier: ChunkMeshifier,
    pub camera: Camera,
    entities: Vec<Entity>,
    physics_engine: PhysicsEngine,
}

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

        let mut world = World::new();
        world.generate_chunk([0, 0, 0]);
        world.generate_chunk([1, 0, 0]);

        let player = Entity {
            id: EntityId::new(),
            pos: [0.5, 20.0, 0.5].into(),
            vel: [0.0, 0.0, 0.0].into(),
            collider: Collider {
                extents: [0.5, 1.8, 0.5].into()
            },
            components: vec![
                Box::new(PlayerController::new())
            ],
        };

        Self {
            world,
            block_registry,
            atlas,
            chunk_meshifier: ChunkMeshifier::new(),
            camera: Camera::new([0.0, 0.0, 0.0], cgmath::Deg(0.0), cgmath::Deg(0.0)),
            entities: vec![player],
            physics_engine: PhysicsEngine::new(),
        }
    }

    pub fn update(&mut self, dt: Duration) {
        let mut commands = Commands::new();

        for entity in &mut self.entities {
            let id = entity.id;
            for component in &mut entity.components {
                component.update(id, dt, &mut commands);
            }
        }

        commands.execute(self);
        self.physics_engine.do_physics(&self.world, &mut self.entities, dt);
    }

    pub fn keyboard_input(&mut self, event: KeyEvent) {
        let mut commands = Commands::new();

        for entity in &mut self.entities {
            let id = entity.id;
            for component in &mut entity.components {
                component.keyboard_input(event.clone(), &mut commands);
            }
        }

        commands.execute(self);
    }

    pub fn mouse_input(&mut self, delta: Vector2<f32>) {
        let mut commands = Commands::new();

        for entity in &mut self.entities {
            let id = entity.id;
            for component in &mut entity.components {
                component.mouse_input(delta, &mut commands);
            }
        }

        commands.execute(self);
    }

    pub fn get_objects_to_render(&mut self, device: &wgpu::Device) -> Vec<Object> {
        self.world
            .chunks
            .iter()
            .map(|chunk| {
                let mesh = self.chunk_meshifier.meshify(
                    &self.world,
                    chunk,
                    &self.atlas,
                    &self.block_registry,
                    device,
                );
                let object = Object::new(
                    mesh,
                    Instance {
                        position: chunk.pos.cast().unwrap().to_vec() * 16.0,
                        rotation: Quaternion::from_angle_z(cgmath::Deg(0.0)),
                    },
                    device,
                );

                object
            })
            .collect()
    }
}
