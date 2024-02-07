use std::{collections::HashMap, io::Write, sync::{atomic::Ordering, Arc}, time::Duration};

use bevy_ecs::{
    component::Component,
    schedule::{Schedule, ScheduleLabel},
    system::{Res, ResMut, Resource},
};
use cgmath::{EuclideanSpace, InnerSpace, Point3, Quaternion, Rotation3, Vector2, Vector3};
use wgpu::{naga::FastHashMap, RenderPass};
use winit::{
    event::{ElementState, KeyEvent, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{
    assets::AssetManager,
    camera::Camera,
    input::Input,
    mesh::{Direction, DrawModel, Mesh, MeshBuilder, MeshVertex},
    meshifier::ChunkMeshifier,
    object::Object,
    Instance,
};

use self::{
    atlas::Atlas,
    block::{BlockAttributes, BlockId, BlockRegistry},
    physics::Collider,
    player::PlayerController,
    world::World,
};

pub mod atlas;
pub mod block;
pub mod chunk;
mod physics;
mod player;
pub mod world;

#[derive(Clone, Copy, PartialEq, Component)]
pub struct Position(pub Point3<f32>);
#[derive(Clone, Copy, PartialEq, Component)]
pub struct Velocity(pub Vector3<f32>);

pub struct Game {
    atlas: Atlas,
    chunk_meshifier: ChunkMeshifier,
    ecs_world: bevy_ecs::world::World,
    block_select_object: Object,
    show_select_object: bool,
    chunk_objects: FastHashMap<Point3<isize>, Object>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ScheduleLabel)]
pub enum ScheduleStage {
    Update,
}

#[derive(Clone, Copy, PartialEq, Resource)]
pub struct DeltaTime(pub f32);

impl Game {
    pub fn new(asset_manager: &mut AssetManager, device: &wgpu::Device) -> Self {
        let material = asset_manager.load_material("assets/atlas.png").unwrap();
        let atlas = Atlas::new(material.clone(), 16);

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

        let neco_arc_block_attr = BlockAttributes {
            transparent: false,
            invisible: false,
            uv_coords: [2, 0].into(),
        };
        block_registry.register(BlockId(3), neco_arc_block_attr);

        let blue_block_attr = BlockAttributes {
            transparent: false,
            invisible: false,
            uv_coords: [3, 0].into(),
        };
        block_registry.register(BlockId(4), blue_block_attr);

        let mut world = World::new();

        for x in -20..=20 {
            for z in -20..=20 {
                for y in -0..=0 {
                    world.generate_chunk([x, y, z]);
                }
            }
        }

        let camera = Camera::new([0.0, 0.0, 0.0], cgmath::Deg(0.0), cgmath::Deg(0.0));

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
        schedule.add_systems({
            #[derive(Resource)]
            struct T(f32, f32);
            ecs_world.insert_resource(T(0.0, 0.0));
            |mut t: ResMut<T>, dt: Res<DeltaTime>| {
                t.0 += dt.0;
                t.1 += 1.0;
                if t.0 >= 1.0 {
                    println!("{} FPS", t.1 / t.0);
                    t.0 -= 1.0;
                    t.1 = 0.0;
                }
            }
        });
        ecs_world.add_schedule(schedule);

        ecs_world.spawn((
            Position([0.0, 20.0, 0.0].into()),
            Velocity([0.0, 0.0, 0.0].into()),
            Collider {
                enabled: false,
                gravity: false,
                extents: [0.5, 1.8, 0.5].into(),
            },
            PlayerController::new(),
        ));

        let block_select_object = Object::new(
            {
                let mut builder = MeshBuilder::new();
                let line_width = 0.025;
                let dist = -line_width / 2.0 + 0.001;
                let far = 0.5 + dist + line_width / 2.0;
                let close = 0.5 + dist - line_width / 2.0;
                let vertices = [
                    // Top line
                    [-far, far],
                    [far, far],
                    [close, close],
                    [-close, close],
                    // Right line
                    [close, close],
                    [far, far],
                    [far, -far],
                    [close, -close],
                    // Bottom line
                    [-close, -close],
                    [close, -close],
                    [far, -far],
                    [-far, -far],
                    // Left line
                    [-far, far],
                    [-close, close],
                    [-close, -close],
                    [-far, -far],
                ];
                let indices = [
                    // Top line
                    0, 3, 1, 1, 3, 2, // Right line
                    4, 7, 5, 5, 7, 6, // Bottom line
                    8, 11, 9, 9, 11, 10, // Left line
                    12, 15, 13, 13, 15, 14,
                ];
                let dirs = [
                    Direction::North,
                    Direction::South,
                    Direction::East,
                    Direction::West,
                    Direction::Up,
                    Direction::Down,
                ];

                for dir in dirs {
                    let vertices = vertices
                        .into_iter()
                        .map(|v| MeshVertex {
                            position: (dir.on_plane(v.into())
                                + dir.normal() * far
                                + Vector3::new(0.5, 0.5, 0.5))
                            .into(),
                            tex_coords: [0.0, 0.0],
                            ambient_occlusion: 1.0,
                            normal: dir.normal().into(),
                        })
                        .collect::<Vec<_>>();
                    builder.add_vert_indices(&vertices, &indices);
                }

                builder.build(material, device).into()
            },
            Instance {
                position: [0.0, 0.0, 0.0].into(),
                rotation: Quaternion::from_angle_z(cgmath::Deg(0.0)),
            },
            device,
        );

        Self {
            atlas,
            chunk_meshifier: ChunkMeshifier::new(),
            ecs_world,
            block_select_object,
            show_select_object: true,
            chunk_objects: FastHashMap::default()
        }
    }

    pub fn update(&mut self, dt: Duration) {
        std::io::stdout().flush().unwrap();
        self.ecs_world.resource_mut::<DeltaTime>().0 = dt.as_secs_f32();
        self.ecs_world.run_schedule(ScheduleStage::Update);
        self.ecs_world.resource_mut::<Input>().end_frame();

        let camera = self.ecs_world.resource::<Camera>();
        let world = self.ecs_world.resource::<World>();
        let block_registry = self.ecs_world.resource::<BlockRegistry>();
        let pos = if let Some(hitinfo) =
            world.raycast(camera.position, camera.forward(), 10000.0, block_registry)
        {
            hitinfo.position.cast().unwrap()
        } else {
            [0.0, 0.0, 0.0].into()
        };
        self.block_select_object
            .edit_instance(|instance| instance.position = pos);
    }

    pub fn keyboard_input(&mut self, event: KeyEvent) {
        if let KeyEvent {
            physical_key: PhysicalKey::Code(KeyCode::KeyF),
            state: ElementState::Pressed,
            repeat: false,
            ..
        } = &event
        {
            self.chunk_meshifier.enable_ao = !self.chunk_meshifier.enable_ao;
            for chunk in self.ecs_world.resource_mut::<World>().chunks.values() {
                chunk
                    .dirty
                    .store(true, Ordering::Relaxed);
            }
        }

        if let KeyEvent {
            physical_key: PhysicalKey::Code(KeyCode::KeyX),
            state: ElementState::Pressed,
            repeat: false,
            ..
        } = &event
        {
            self.show_select_object = !self.show_select_object;
        }

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
        self.ecs_world
            .resource_mut::<Input>()
            .process_mouse_input(button, state);
    }

    pub fn camera(&self) -> &Camera {
        self.ecs_world.resource::<Camera>()
    }

    pub fn get_objects_to_render(&mut self, device: &wgpu::Device) -> impl Iterator<Item=&mut Object> {
        let world = self.ecs_world.resource::<World>();
        let block_registry = self.ecs_world.resource::<BlockRegistry>();

        for chunk in world.chunks.values() {
            if chunk.dirty.load(Ordering::Relaxed) || !self.chunk_objects.contains_key(&chunk.pos) {
                let mesh = self.chunk_meshifier.meshify(world, chunk, &self.atlas, block_registry, device);
                let object = Object::new(mesh, Instance {
                    position: chunk.pos.cast::<f32>().unwrap() * 16.0,
                    rotation: Quaternion::from_angle_z(cgmath::Deg(0.0)),
                }, device);
                self.chunk_objects.insert(chunk.pos, object);
            }
        }
        
        if self.show_select_object {
            // objects.push(Object::new(self.block_select_object.mesh.clone(), self.block_select_object.instance().clone(), device));
        }
        
        self.chunk_objects.values_mut()
    }
}
