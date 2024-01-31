use super::{
    block::{Block, BlockId, BlockMetadata},
    chunk::Chunk,
};
use cgmath::Point3;
use rand::Rng;

pub struct World {
    pub chunks: Vec<Chunk>,
}

impl World {
    pub fn new() -> Self {
        Self {
            chunks: vec![],
        }
    }

    pub fn generate_chunk(&mut self, pos: impl Into<Point3<isize>>) {
        let mut chunk = Chunk::new(pos.into());

        for (_, block) in chunk.block_iter_mut() {
            block.id.0 = rand::thread_rng().gen::<u8>() % 3;
        }

        self.chunks.push(chunk);
    }

    pub fn delete_chunk(&mut self, pos: impl Into<Point3<isize>>) {
        let pos = pos.into();
        if let Some(i) = self.chunks.iter().position(|c| c.pos == pos) {
            self.chunks.swap_remove(i);
        }
    }
}
