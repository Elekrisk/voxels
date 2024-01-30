use super::{
    block::{Block, BlockId, BlockMetadata},
    chunk::Chunk,
};
use rand::Rng;

pub struct World {
    pub chunks: Vec<Chunk>,
}

impl World {
    pub fn new() -> Self {
        let mut chunk = Chunk {
            blocks: [[[Block {
                id: BlockId(0),
                metadata: BlockMetadata(0),
            }; _]; _]; _],
        };

        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let is_dirt = rand::thread_rng().gen();
                    if is_dirt {
                        chunk.blocks[x][y][z].id.0 = 1;
                    }
                }
            }
        }

        Self {
            chunks: vec![chunk],
        }
    }
}
