use std::{
    ops::{Add, AddAssign},
    sync::atomic::{AtomicBool, AtomicUsize},
};

use cgmath::{num_traits::Euclid, EuclideanSpace, Point3, Vector3};

use crate::mesh::{D3Accessible, Vector3Accessor};

use super::block::Block;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos(Point3<isize>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPos(Point3<isize>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkRelativeBlockPos(Point3<usize>);

impl From<Point3<isize>> for ChunkPos {
    fn from(value: Point3<isize>) -> Self {
        Self(value)
    }
}

impl Add<Vector3<isize>> for ChunkPos {
    type Output = Self;

    fn add(self, rhs: Vector3<isize>) -> Self::Output {
        (Point3::from(self) + rhs).into()
    }
}

impl AddAssign<Vector3<isize>> for ChunkPos {
    fn add_assign(&mut self, rhs: Vector3<isize>) {
        *self = *self + rhs;
    }
}

impl From<ChunkPos> for Point3<isize> {
    fn from(value: ChunkPos) -> Self {
        value.0
    }
}

impl ChunkPos {
    pub fn center(self) -> Point3<f32> {
        Point3::from(self + ChunkRelativeBlockPos::from(Point3::from([0; 3])))
            .cast()
            .unwrap()
            + Vector3::from([Chunk::SIZE as f32 / 2.0; 3])
    }
}

impl From<Point3<isize>> for BlockPos {
    fn from(value: Point3<isize>) -> Self {
        Self(value)
    }
}

impl From<BlockPos> for Point3<isize> {
    fn from(value: BlockPos) -> Self {
        value.0
    }
}

impl From<Point3<usize>> for ChunkRelativeBlockPos {
    fn from(value: Point3<usize>) -> Self {
        Self(value)
    }
}

impl From<ChunkRelativeBlockPos> for Point3<usize> {
    fn from(value: ChunkRelativeBlockPos) -> Self {
        value.0
    }
}

impl BlockPos {
    pub fn from_point(point: Point3<f32>) -> Self {
        Self(point.map(|e| e.floor() as isize))
    }

    pub fn chunk_pos(self) -> ChunkPos {
        self.0.map(|e| e.div_euclid(Chunk::SIZE as _)).into()
    }

    pub fn rel_pos(self) -> ChunkRelativeBlockPos {
        self.0.map(|e| e.rem_euclid(Chunk::SIZE as _) as _).into()
    }
}

impl Add<ChunkRelativeBlockPos> for ChunkPos {
    type Output = BlockPos;

    fn add(self, rhs: ChunkRelativeBlockPos) -> Self::Output {
        (self.0 * Chunk::SIZE as isize + rhs.0.cast().unwrap().to_vec()).into()
    }
}

impl D3Accessible for ChunkPos {
    type Element = isize;

    fn get(self, accessor: Vector3Accessor) -> Self::Element {
        match accessor {
            Vector3Accessor::X => self.0.x,
            Vector3Accessor::Y => self.0.y,
            Vector3Accessor::Z => self.0.z,
        }
    }

    fn set(&mut self, accessor: Vector3Accessor, value: Self::Element) {
        match accessor {
            Vector3Accessor::X => self.0.x = value,
            Vector3Accessor::Y => self.0.y = value,
            Vector3Accessor::Z => self.0.z = value,
        }
    }
}

impl D3Accessible for BlockPos {
    type Element = isize;

    fn get(self, accessor: Vector3Accessor) -> Self::Element {
        match accessor {
            Vector3Accessor::X => self.0.x,
            Vector3Accessor::Y => self.0.y,
            Vector3Accessor::Z => self.0.z,
        }
    }

    fn set(&mut self, accessor: Vector3Accessor, value: Self::Element) {
        match accessor {
            Vector3Accessor::X => self.0.x = value,
            Vector3Accessor::Y => self.0.y = value,
            Vector3Accessor::Z => self.0.z = value,
        }
    }
}

impl D3Accessible for ChunkRelativeBlockPos {
    type Element = usize;

    fn get(self, accessor: Vector3Accessor) -> Self::Element {
        match accessor {
            Vector3Accessor::X => self.0.x,
            Vector3Accessor::Y => self.0.y,
            Vector3Accessor::Z => self.0.z,
        }
    }

    fn set(&mut self, accessor: Vector3Accessor, value: Self::Element) {
        match accessor {
            Vector3Accessor::X => self.0.x = value,
            Vector3Accessor::Y => self.0.y = value,
            Vector3Accessor::Z => self.0.z = value,
        }
    }
}

pub struct Chunk {
    pub dirty: AtomicBool,
    pub pos: ChunkPos,
    pub blocks: [[[Block; 16]; 16]; 16],
}

impl Chunk {
    pub const SIZE: usize = 16;

    pub fn new(pos: ChunkPos) -> Self {
        Self {
            dirty: AtomicBool::new(true),
            pos,
            blocks: Default::default(),
        }
    }

    pub fn block(&self, pos: ChunkRelativeBlockPos) -> &Block {
        let pos = pos.0;
        &self.blocks[pos.x][pos.y][pos.z]
    }

    pub fn block_mut(&mut self, pos: ChunkRelativeBlockPos) -> &mut Block {
        let pos = pos.0;
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

    pub fn get_dirty(&self) -> bool {
        self.dirty.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_dirty(&self, dirty: bool) {
        self.dirty
            .store(dirty, std::sync::atomic::Ordering::Relaxed);
    }
}
