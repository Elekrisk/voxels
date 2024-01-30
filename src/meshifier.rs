use cgmath::{Vector2, Vector3, Zero};

use crate::{
    game::{
        block::{BlockId, BlockRegistry},
        chunk::Chunk,
    },
    mesh::{Mesh, MeshBuilder},
};

pub struct Meshifier {}

impl Meshifier {
    pub fn meshify(chunk: &Chunk, block_registry: &BlockRegistry, device: &wgpu::Device) -> Mesh {
        let mut builder = MeshBuilder::new();

        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let block = &chunk.blocks[x][y][z];
                    let attr = block_registry.get(block.id).unwrap();
                    if attr.invisible {
                        continue;
                    }

                    let offset = Vector3::new(x as _, y as _, z as _);

                    builder.add_face(offset, crate::mesh::Direction::North);
                    builder.add_face(offset, crate::mesh::Direction::South);
                    builder.add_face(offset, crate::mesh::Direction::East);
                    builder.add_face(offset, crate::mesh::Direction::West);
                    builder.add_face(offset, crate::mesh::Direction::Up);
                    builder.add_face(offset, crate::mesh::Direction::Down);
                }
            }
        }

        let material = block_registry.get(BlockId(0)).unwrap().material.clone();

        builder.build(material, device)
    }
}
