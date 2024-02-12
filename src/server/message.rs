use cgmath::Vector3;
use serde::{Deserialize, Serialize};

use crate::game::{block::Block, chunk::{BlockPos, Chunk, ChunkPos}};


#[derive(Debug, Serialize, Deserialize)]
pub enum MessageToServer {
    Connect,
    UpdatePlayerPosition {
        new_position: Vector3<f32>,
    },
    GetChunk(ChunkPos),
    GetChunks(Vec<ChunkPos>),
    UnloadChunks(Vec<ChunkPos>),
    ReplaceBlock {
        pos: BlockPos,
        new_block: Block
    },
}

impl MessageToServer {
    pub fn name(&self) -> &'static str {
        match self {
            MessageToServer::Connect => "MessageToServer::Connect",
            MessageToServer::UpdatePlayerPosition { .. } => "MessageToServer::UpdatePlayerPosition",
            MessageToServer::GetChunk(_) => "MessageToServer::GetChunk",
            MessageToServer::GetChunks(_) => "MessageToServer::GetChunks",
            MessageToServer::UnloadChunks(_) => "MessageToServer::UnloadChunks",
            MessageToServer::ReplaceBlock { pos, new_block } => "MessageToServer::ReplaceBlock",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageToClient {
    Ok,
    EntitiesPositionUpdate {
        entity: usize,
        new_position: Vector3<f32>,
    },
    Chunk(Chunk),
    Chunks(Vec<Chunk>),
    BlockPlaced {
        pos: BlockPos,
        new_block: Block,
    }
}

impl MessageToClient {
    pub fn name(&self) -> &'static str {
        match self {
            MessageToClient::Ok => "MessageToClient::Ok",
            MessageToClient::EntitiesPositionUpdate { .. } => "MessageToClient::EntitiesPositionUpdate",
            MessageToClient::Chunk(_) => "MessageToClient::Chunk",
            MessageToClient::Chunks(_) => "MessageToClient::Chunks",
            MessageToClient::BlockPlaced { .. } => "MessageToClient::BlockPlaced",
        }
    }
}
