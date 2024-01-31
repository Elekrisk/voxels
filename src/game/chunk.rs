use std::sync::atomic::AtomicUsize;

use cgmath::Point3;

use super::block::Block;

pub struct Chunk {
    pub pos: Point3<isize>,
    pub blocks: [[[Block; 16]; 16]; 16],
}

impl Chunk {
    pub fn new(pos: Point3<isize>) -> Self {
        Self {
            pos,
            blocks: Default::default(),
        }
    }

    pub fn block(&self, pos: impl Into<Point3<usize>>) -> &Block {
        let pos = pos.into();
        &self.blocks[pos.x][pos.y][pos.z]
    }

    pub fn block_mut(&mut self, pos: impl Into<Point3<usize>>) -> &mut Block {
        let pos = pos.into();
        &mut self.blocks[pos.x][pos.y][pos.z]
    }

    pub fn block_iter(&self) -> impl Iterator<Item = (Point3<usize>, &Block)> {
        self.blocks.iter().enumerate().flat_map(|(x, d2)| {
            d2.iter().enumerate().flat_map(move |(y, d1)| {
                d1.iter()
                    .enumerate()
                    .map(move |(z, b)| ([x, y, z].into(), b))
            })
        })
    }

    pub fn block_iter_mut(&mut self) -> impl Iterator<Item = (Point3<usize>, &mut Block)> {
        self.blocks.iter_mut().enumerate().flat_map(|(x, d2)| {
            d2.iter_mut().enumerate().flat_map(move |(y, d1)| {
                d1.iter_mut()
                    .enumerate()
                    .map(move |(z, b)| ([x, y, z].into(), b))
            })
        })
    }
}
