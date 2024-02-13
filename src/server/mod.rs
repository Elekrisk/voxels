use std::{
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use async_std::channel::{Receiver, Sender, TryRecvError};
use bevy_ecs::schedule::{Schedule, ScheduleLabel};
use cgmath::Point3;
use futures::{FutureExt, StreamExt};
use itertools::Itertools;
use quinn::{Endpoint, RecvStream, ServerConfig};
use rusqlite::OptionalExtension;
use uuid::Uuid;
use wgpu::naga::{FastHashMap, FastHashSet};

use self::{
    super::game::{
        chunk::{Chunk, ChunkPos},
        world::World,
    },
    connection::{Connection, RemoteTransport, Respond, Transport},
    message::{MessageToClient, MessageToServer},
};

pub mod connection;
pub mod message;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ScheduleLabel)]
pub enum ScheduleStage {
    Tick,
}

pub struct Server {
    ecs_world: bevy_ecs::world::World,
    endpoint: Endpoint,
    connections: FastHashMap<
        Uuid,
        (
            Connection,
            Receiver<(MessageToServer, Respond<MessageToClient>)>,
        ),
    >,
    loaded_chunks: FastHashMap<ChunkPos, usize>,
    player_loaded_chunks: FastHashMap<Uuid, FastHashSet<ChunkPos>>,
    db: rusqlite::Connection,
    shutdown_signal: Receiver<()>,
}

impl Server {
    pub fn new(shutdown_signal: Receiver<()>) -> Self {
        let server_config = rustls::ServerConfig::builder();
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
        let key = rustls::PrivateKey(cert.serialize_private_key_der().into());
        let cert = rustls::Certificate(cert.serialize_der().unwrap());
        let config = server_config
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .unwrap();
        let endpoint = quinn::Endpoint::server(
            ServerConfig::with_crypto(Arc::new(config)),
            ("::".parse::<IpAddr>().unwrap(), 1234).into(),
        )
        .unwrap();

        let db = rusqlite::Connection::open("./savegame.db3").unwrap();
        db.execute(
            "
        CREATE TABLE IF NOT EXISTS chunks (
            pos BLOB NOT NULL PRIMARY KEY,
            blocks BLOB NOT NULL
        );
        ",
            [],
        )
        .unwrap();

        let world = World::new();

        let mut ecs_world = bevy_ecs::world::World::new();

        ecs_world.insert_resource(world);

        Self {
            ecs_world,
            endpoint,
            connections: FastHashMap::default(),
            loaded_chunks: FastHashMap::default(),
            player_loaded_chunks: FastHashMap::default(),
            db,
            shutdown_signal
        }
    }

    fn shutdown(&mut self) {
        for &pos in self.loaded_chunks.keys() {
            let mut world = self.ecs_world.resource_mut::<World>();
            let chunk = world.chunks.remove(&pos).unwrap();
            self.db
                .execute(
                    "INSERT OR REPLACE INTO chunks (pos, blocks) VALUES(?2, ?1);",
                    (chunk.serialize(), pos),
                )
                .unwrap();
        }

        self.loaded_chunks.clear();
        self.player_loaded_chunks.clear();
        self.connections.clear();
    }

    fn clean_up_disconnected_player(&mut self, player: Uuid) {
        if let Some(loaded_chunks) = self.player_loaded_chunks.remove(&player) {
            for pos in loaded_chunks {
                let count = self.loaded_chunks.get_mut(&pos).unwrap();
                *count -= 1;
                if *count == 0 {
                    self.loaded_chunks.remove(&pos);
                    let mut world = self.ecs_world.resource_mut::<World>();
                    let chunk = world.chunks.remove(&pos).unwrap();
                    self.db
                        .execute(
                            "INSERT OR REPLACE INTO chunks (pos, blocks) VALUES(?2, ?1);",
                            (chunk.serialize(), pos),
                        )
                        .unwrap();
                }
            }
        }

        self.connections.remove(&player);
    }

    pub async fn run(&mut self) {
        let endpoint = self.endpoint.clone();

        let (tx, rx) = async_std::channel::unbounded();
        let mut rx = rx.fuse();

        async_std::task::spawn(accept(endpoint, tx));

        let mut tick_interval =
            async_std::stream::interval(Duration::from_secs_f32(1.0 / 20.0)).fuse();

        let receiver = &self.shutdown_signal.clone();
        let mut shutdown = receiver.recv().fuse();

        let mut now = Instant::now();

        let mut tot = 0.0;
        let mut runs = 0;
        // let (send_to_server, recv_to_server) = async_std::channel::unbounded();

        println!("Starting listening serverside...");
        loop {
            futures::select! {
                x = rx.next() => match x {
                    Some(conn) => {
                        println!("Connection received from {}", conn.player_id);
                        let transport = conn.transport.clone();

                        // Incoming messages are sent over this channel
                        let (send_to_server, recv_to_server) = async_std::channel::unbounded();
                        // Spawn task that constantly reads messages from the player
                        async_std::task::spawn(async move {
                            read_messages(transport, conn.player_id, send_to_server).await.unwrap();
                        });

                        self.player_loaded_chunks.insert(conn.player_id, FastHashSet::default());
                        self.connections.insert(conn.player_id, (conn, recv_to_server));
                    },
                    None => {
                        break;
                    }
                },
                _ = tick_interval.next() => {

                    let n = Instant::now();
                    let diff = n - now;
                    tot += diff.as_secs_f32();
                    runs += 1;
                    // println!("{:.4} - {:.8} - {:.4}", diff.as_secs_f32(), tot / runs as f32, runs as f32 / tot);
                    now = n;


                    self.tick().await;
                }
                _ = shutdown => {
                    self.shutdown();
                    break;
                }
            }
        }
    }

    pub async fn tick(&mut self) {
        // We collect all incoming messages into a vec, so that we can avoid having `self` borrowed while
        // we do stuff with them
        let mut msgs = vec![];

        let mut disconnected_players = vec![];

        for (conn, rx) in self.connections.values() {
            loop {
                match rx.try_recv() {
                    Ok(x) => msgs.push((conn.player_id, x)),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Closed) => {
                        disconnected_players.push(conn.player_id);
                        break;
                    },
                }
            }
        }
        
        for player in disconnected_players {
            println!("Player disconnected! Cleaning up");
            self.clean_up_disconnected_player(player);
        }

        for (player_id, (msg, mut respond)) in msgs {
            match msg {
                MessageToServer::Connect => {}
                MessageToServer::UpdatePlayerPosition { new_position } => {}
                MessageToServer::GetChunks(chunks) => {
                    let mut chunks = chunks
                        .into_iter()
                        .map(|pos| self.load_chunk(player_id, pos));

                    loop {
                        // Batch chunks so that the client can start rendering before all are sent over the network
                        // Currently uses a batch number so large that it will never happen, due to it being fast enough already
                        let chunks = chunks.by_ref().take(100000000000).collect::<Vec<_>>();
                        if chunks.is_empty() {
                            break;
                        }

                        respond
                            .respond(&MessageToClient::Chunks(chunks))
                            .await
                            .unwrap();
                    }
                }
                MessageToServer::UnloadChunks(chunks) => {
                    for pos in chunks {
                        self.unload_chunk(player_id, pos);
                    }
                }
                MessageToServer::ReplaceBlock { pos, new_block } => {
                    let chunk_pos = pos.chunk_pos();
                    let rel_pos = pos.rel_pos();
                    let mut world = self.ecs_world.resource_mut::<World>();
                    // If the chunk is loaded, directly modify it
                    if let Some(chunk) = world.chunk_mut(chunk_pos) {
                        *chunk.block_mut(rel_pos) = new_block;
                    } else {
                        // Else, load it, modify it, and then unload it
                        self.load_chunk(player_id, chunk_pos);
                        let mut world = self.ecs_world.resource_mut::<World>();
                        let chunk = world.chunk_mut(chunk_pos).unwrap();
                        *chunk.block_mut(rel_pos) = new_block;
                        self.unload_chunk(player_id, chunk_pos);
                    };

                    // Propagate block placements to all connected players
                    for (player, (conn, _)) in &self.connections {
                        if *player == player_id {
                            println!("Skipping sending to {player}");
                            continue;
                        }
                        println!("Sending to {player}");

                        conn.transport
                            .transact::<_, ()>(&MessageToClient::BlockPlaced { pos, new_block })
                            .await
                            .unwrap();
                    }
                }
            }
        }
    }

    /// Loads a chunk, or generates it if no such chunk exists
    pub fn load_chunk(&mut self, loader: Uuid, pos: ChunkPos) -> Chunk {
        // Add this chunk to the list of chunks that `loader` has loaded
        let player_loaded = self.player_loaded_chunks.get_mut(&loader).unwrap();
        let just_loaded = player_loaded.insert(pos);

        if just_loaded {
            // Increment the "reference counter" for the chunk
            *self.loaded_chunks.entry(pos).or_default() += 1;
        }

        let world = self.ecs_world.resource::<World>();

        if let Some(chunk) = world.chunk(pos) {
            println!("Loading chunk {pos:?} from already loaded");
            chunk.clone()
        } else if let Some(chunk) = self
            .db
            .query_row("SELECT blocks FROM chunks WHERE pos = ?1", (pos,), |row| {
                Ok(Chunk::deserialize(row.get_ref(0)?.as_blob()?))
            })
            .optional()
            .unwrap()
        {
            println!("Loading chunk {pos:?} from database");
            let mut world = self.ecs_world.resource_mut::<World>();
            world.chunks.insert(pos, chunk.clone());
            chunk
        } else {
            println!("Loading chunk {pos:?} from newly generated");
            let mut world = self.ecs_world.resource_mut::<World>();
            let chunk = world.worldgen.generate_chunk(pos);
            world.chunks.insert(pos, chunk.clone());
            chunk
        }
    }

    /// Unload a chunk.
    /// Only actually unloads it when no player wants this loaded anymore.
    pub fn unload_chunk(&mut self, loader: Uuid, pos: ChunkPos) {
        let player_loaded = self.player_loaded_chunks.get_mut(&loader).unwrap();
        let was_loaded = player_loaded.remove(&pos);

        if was_loaded {
            let count = self.loaded_chunks.get_mut(&pos).unwrap();
            *count -= 1;
            if *count == 0 {
                self.loaded_chunks.remove(&pos);
                let mut world = self.ecs_world.resource_mut::<World>();
                let chunk = world.chunks.remove(&pos).unwrap();
                self.db
                    .execute(
                        "INSERT OR REPLACE INTO chunks (pos, blocks) VALUES(?2, ?1);",
                        (chunk.serialize(), pos),
                    )
                    .unwrap();
            }
        }
    }
}

/// Wait for incoming connections, sending them through the channel
async fn accept(endpoint: Endpoint, tx: async_std::channel::Sender<Connection>) {
    loop {
        if let Some(connecting) = endpoint.accept().await {
            let Ok(x) = connecting.await else { continue };

            let player_id = Uuid::new_v4();

            let conn = Connection {
                player_id,
                transport: Transport::Remote(RemoteTransport { connection: x }),
            };

            tx.send(conn).await.unwrap();
        }
    }
}

/// Reads messages from a player and sends them through the channel
async fn read_messages(
    mut transport: Transport,
    uuid: Uuid,
    tx: Sender<(MessageToServer, Respond<MessageToClient>)>,
) -> anyhow::Result<()> {
    loop {
        let (msg, respond) = transport
            .accept_transact::<MessageToServer, MessageToClient>()
            .await?;
        tx.send((msg, respond)).await?;
    }
}
