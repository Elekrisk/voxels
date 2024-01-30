use super::block::Block;

pub struct Chunk {
    pub blocks: [[[Block; 16]; 16]; 16],
}
