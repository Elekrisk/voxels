use std::sync::Arc;

use bevy_ecs::system::Resource;
use cgmath::Point2;

use crate::mesh::Material;

#[derive(Debug, Clone, Copy)]
pub struct BlockId(pub u8);
#[derive(Debug, Clone, Copy)]
pub struct BlockMetadata(pub u8);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Block {
    pub id: BlockId,
    pub metadata: BlockMetadata,
}

impl Default for Block {
    fn default() -> Self {
        Self { id: BlockId(0), metadata: BlockMetadata(0) }
    }
}

pub trait BlockInfo {}

pub struct BlockAttributes {
    pub transparent: bool,
    pub invisible: bool,
    pub uv_coords: Point2<usize>,
}

#[derive(Resource)]
pub struct BlockRegistry {
    blocks: [Option<BlockAttributes>; 256],
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            blocks: [const { None }; _],
        }
    }

    pub fn register(&mut self, id: BlockId, info: BlockAttributes) -> Option<()> {
        if self.blocks[id.0 as usize].is_some() {
            None
        } else {
            self.blocks[id.0 as usize] = Some(info);
            Some(())
        }
    }

    pub fn get(&self, id: BlockId) -> Option<&BlockAttributes> {
        self.blocks[id.0 as usize].as_ref()
    }
}
