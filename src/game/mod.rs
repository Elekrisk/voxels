use std::{
    collections::HashMap,
    io::Write,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use async_std::channel::{Receiver, Sender};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    schedule::{Schedule, ScheduleLabel},
    system::{Res, ResMut, Resource},
};
use cgmath::{EuclideanSpace, InnerSpace, Point3, Quaternion, Rotation3, Vector2, Vector3};
use futures::{pin_mut, TryStreamExt};
use quinn::{Endpoint, TransportConfig};
use wgpu::{
    naga::{FastHashMap, FastHashSet},
    RenderPass,
};
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
    server::{
        connection::{RemoteTransport, Respond, SkipServerVerification, Transaction, Transport},
        message::{MessageToClient, MessageToServer},
    },
    Instance,
};

use self::{
    atlas::Atlas,
    block::{BlockAttributes, BlockId, BlockRegistry},
    chunk::{BlockPos, Chunk, ChunkPos},
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
mod worldgen;

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
    chunk_objects: FastHashMap<ChunkPos, Object>,
    chunk_loading_distance: isize,
    server_connection: Transport,
    load_chunk_tx: Sender<Transaction<MessageToClient>>,
    chunk_loaded_rx: Receiver<Vec<Chunk>>,
    loading_chunks: FastHashSet<ChunkPos>,
    msg_queue_rx: Receiver<MessageToServer>,
    msg_from_server_rx: Receiver<(MessageToClient, Respond<MessageToServer>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ScheduleLabel)]
pub enum ScheduleStage {
    Update,
}

#[derive(Clone, Copy, PartialEq, Resource)]
pub struct DeltaTime(pub f32);

#[derive(Clone, Resource)]
pub struct MessageQueue(Sender<MessageToServer>);

impl Game {
    pub async fn new(asset_manager: &mut AssetManager, device: &wgpu::Device) -> Self {
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

        // for x in -0..=0 {
        //     for z in -0..=0 {
        //         for y in -0..=0 {
        //             world.generate_chunk(Point3::new(x, y, z).into());
        //         }
        //     }
        // }

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
                    // println!("{} FPS", t.1 / t.0);
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
                let line_width = 0.01;
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

        let client = quinn::Endpoint::client("[::]:0".parse().unwrap()).unwrap();
        let client_config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();
        let mut tc = TransportConfig::default();
        tc.keep_alive_interval(Some(Duration::from_secs_f32(5.0).try_into().unwrap()));
        let mut client_config = quinn::ClientConfig::new(Arc::new(client_config));
        client_config.transport_config(Arc::new(tc));
        println!("Connecting...");
        let connection = client
            .connect_with(client_config, "[::1]:1234".parse().unwrap(), "localhost")
            .unwrap()
            .await
            .unwrap();
        println!("Connected!");

        let (load_chunk_tx, load_chunk_rx) =
            async_std::channel::unbounded::<Transaction<MessageToClient>>();
        let (chunk_loaded_tx, chunk_loaded_rx) = async_std::channel::unbounded::<Vec<Chunk>>();

        async_std::task::spawn(async move {
            loop {
                let mut transaction = load_chunk_rx.recv().await.unwrap();
                let tx = chunk_loaded_tx.clone();
                async_std::task::spawn(async move {
                    let stream = transaction.stream();
                    pin_mut!(stream);
                    while let Ok(Some(msg)) = stream.try_next().await {
                        let chunks = match msg {
                            MessageToClient::Chunk(chunk) => vec![chunk],
                            MessageToClient::Chunks(chunks) => chunks,
                            _ => unreachable!(),
                        };
                        tx.send(chunks).await.unwrap();
                    }
                });
            }
        });

        let (msg_from_server_tx, msg_from_server_rx) = async_std::channel::unbounded();
        let transport = Transport::Remote(RemoteTransport { connection });
        let mut tp = transport.clone();
        async_std::task::spawn(async move {
            loop {
                let (msg, respond) = tp.accept_transact::<MessageToClient, MessageToServer>().await.unwrap();

                msg_from_server_tx.send((msg, respond)).await.unwrap();
            }
        });

        let (msg_queue_tx, msg_queue_rx) = async_std::channel::unbounded();
        ecs_world.insert_resource(MessageQueue(msg_queue_tx));

        Self {
            atlas,
            chunk_meshifier: ChunkMeshifier::new(),
            ecs_world,
            block_select_object,
            show_select_object: true,
            chunk_objects: FastHashMap::default(),
            chunk_loading_distance: 5,
            server_connection: transport,
            load_chunk_tx,
            chunk_loaded_rx,
            loading_chunks: FastHashSet::default(),
            msg_queue_rx,
            msg_from_server_rx,
        }
    }

    pub async fn update(&mut self, dt: Duration) {
        std::io::stdout().flush().unwrap();
        self.ecs_world.resource_mut::<DeltaTime>().0 = dt.as_secs_f32();
        self.ecs_world.run_schedule(ScheduleStage::Update);
        self.ecs_world.resource_mut::<Input>().end_frame();

        let player_pos = self
            .ecs_world
            .query::<(&Position, &PlayerController)>()
            .single(&self.ecs_world)
            .0
             .0;
        let world = &mut self.ecs_world.resource_mut::<World>();

        while let Ok((msg, respond)) = self.msg_from_server_rx.try_recv() {
            match msg {
                MessageToClient::Ok => todo!(),
                MessageToClient::EntitiesPositionUpdate { entity, new_position } => todo!(),
                MessageToClient::Chunk(_) => todo!(),
                MessageToClient::Chunks(_) => todo!(),
                MessageToClient::BlockPlaced { pos, new_block } => {
                    world.place_block(new_block, pos);
                },
            }
        }

        while let Ok(x) = self.msg_queue_rx.try_recv() {
            self.server_connection.transact::<_, ()>(&x).await.unwrap();
        }

        let mut chunks_to_destroy = vec![];

        let allowed_distance = Chunk::SIZE as f32
            * Chunk::SIZE as f32
            * self.chunk_loading_distance as f32
            * self.chunk_loading_distance as f32;

        for chunk in world.chunks.values() {
            let pos = chunk.pos.center();
            let dist2 = (pos - player_pos).magnitude2();

            if dist2 > allowed_distance {
                chunks_to_destroy.push(chunk.pos);
            }
        }

        if !chunks_to_destroy.is_empty() {
            self.server_connection
                .transact::<_, ()>(&MessageToServer::UnloadChunks(chunks_to_destroy.clone()))
                .await
                .unwrap();
        }

        for chunk_pos in chunks_to_destroy {
            world.delete_chunk(chunk_pos);
            self.chunk_objects.remove(&chunk_pos);
            for dir in Direction::ALL {
                if let Some(chunk) = world.chunk(chunk_pos + dir.normal()) {
                    chunk.set_dirty(true);
                }
            }
        }

        let player_chunk_pos = BlockPos::from_point(player_pos).chunk_pos();

        let mut chunks_to_load = vec![];
        for x in -self.chunk_loading_distance..=self.chunk_loading_distance {
            for y in -self.chunk_loading_distance..=self.chunk_loading_distance {
                for z in -self.chunk_loading_distance..=self.chunk_loading_distance {
                    let chunk_pos =
                        ChunkPos::from(Point3::from(player_chunk_pos) + Vector3::from([x, y, z]));
                    if world.chunk(chunk_pos).is_some() || self.loading_chunks.contains(&chunk_pos)
                    {
                        continue;
                    }
                    let center = chunk_pos.center();

                    let dist2 = (center - player_pos).magnitude2();

                    if dist2 <= allowed_distance {
                        // world.generate_chunk(chunk_pos);
                        chunks_to_load.push(chunk_pos);
                    }
                }
            }
        }

        chunks_to_load.sort_by(|a, b| {
            let dista = (a.center() - player_pos).magnitude2();
            let distb = (b.center() - player_pos).magnitude2();
            dista.partial_cmp(&distb).unwrap()
        });

        if !chunks_to_load.is_empty() {
            self.loading_chunks.extend(&chunks_to_load);
            let chunk_load = self
                .server_connection
                .transact::<MessageToServer, MessageToClient>(&MessageToServer::GetChunks(
                    chunks_to_load,
                ))
                .await
                .unwrap();
            self.load_chunk_tx.send_blocking(chunk_load).unwrap();
        }

        while let Ok(chunks) = self.chunk_loaded_rx.try_recv() {
            for chunk in chunks {
                self.loading_chunks.remove(&chunk.pos);
                world.chunks.insert(chunk.pos, chunk);
            }
        }

        let camera = self.ecs_world.resource::<Camera>();
        let world = self.ecs_world.resource::<World>();
        let block_registry = self.ecs_world.resource::<BlockRegistry>();
        let pos = if let Some(hitinfo) =
            world.raycast(camera.position, camera.forward(), 5.0, block_registry)
        {
            Point3::from(hitinfo.position).cast().unwrap()
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
                chunk.dirty.store(true, Ordering::Relaxed);
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

        if let KeyEvent {
            text: Some(text),
            state: ElementState::Pressed,
            repeat: false,
            ..
        } = &event
            && text == "-"
        {
            self.chunk_loading_distance = 1.max(self.chunk_loading_distance - 1);
        }

        if let KeyEvent {
            text: Some(text),
            state: ElementState::Pressed,
            repeat: false,
            ..
        } = &event
            && text == "+"
        {
            self.chunk_loading_distance += 1;
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

    pub fn get_objects_to_render(
        &mut self,
        device: &wgpu::Device,
    ) -> impl Iterator<Item = &mut Object> {
        let world = self.ecs_world.resource::<World>();
        let block_registry = self.ecs_world.resource::<BlockRegistry>();

        for chunk in world.chunks.values() {
            if chunk.get_dirty() || !self.chunk_objects.contains_key(&chunk.pos) {
                let mesh =
                    self.chunk_meshifier
                        .meshify(world, chunk, &self.atlas, block_registry, device);
                let object = Object::new(
                    mesh,
                    Instance {
                        position: Point3::from(chunk.pos).cast::<f32>().unwrap() * 16.0,
                        rotation: Quaternion::from_angle_z(cgmath::Deg(0.0)),
                    },
                    device,
                );
                self.chunk_objects.insert(chunk.pos, object);
            }
        }

        let mut extra = vec![];

        if self.show_select_object {
            extra.push(&mut self.block_select_object);
        }

        self.chunk_objects.values_mut().chain(extra)
    }
}
