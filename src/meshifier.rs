use std::{cell::RefCell, sync::Arc};

use cgmath::{Point3, Vector2, Vector3, Zero};
use wgpu::naga::FastHashMap;

use crate::{
    game::{
        atlas::Atlas, block::{BlockId, BlockRegistry}, chunk::Chunk, world::World
    },
    mesh::{Mesh, MeshBuilder},
};

pub struct ChunkMeshifier {
    cache: FastHashMap<Point3<isize>, Arc<Mesh>>
}

impl ChunkMeshifier {
    pub fn new() -> Self {
        Self {
            cache: FastHashMap::default(),
        }
    }

    pub fn meshify(
        &mut self,
        world: &World,
        chunk: &Chunk,
        atlas: &Atlas,
        block_registry: &BlockRegistry,
        device: &wgpu::Device,
    ) -> Arc<Mesh> {
        if !chunk.dirty.load(std::sync::atomic::Ordering::Relaxed) && self.cache.contains_key(&chunk.pos) {
            return self.cache.get(&chunk.pos).unwrap().clone();
        }
        
        let mut builder = MeshBuilder::new();

        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let block = &chunk.blocks[x][y][z];
                    let attr = block_registry.get(block.id).unwrap();
                    if attr.invisible {
                        continue;
                    }

                    let offset = Vector3::new(x as _, y as _, z as _) + Vector3::new(0.5, 0.5, 0.5);

                    let is_transparent = |dx: isize, dy: isize, dz: isize| {
                        block_registry
                            .get(
                                chunk.blocks[(x as isize + dx) as usize]
                                    [(y as isize + dy) as usize]
                                    [(z as isize + dz) as usize]
                                    .id,
                            )
                            .unwrap()
                            .transparent
                    };

                    let uv = atlas.uv(attr.uv_coords);

                    if z == 15 || is_transparent(0, 0, 1) {
                        builder.add_face(offset, crate::mesh::Direction::North, uv);
                    }
                    if z == 0 || is_transparent(0, 0, -1) {
                        builder.add_face(offset, crate::mesh::Direction::South, uv);
                    }
                    if y == 15 || is_transparent(0, 1, 0) {
                        builder.add_face(offset, crate::mesh::Direction::Up, uv);
                    }
                    if y == 0 || is_transparent(0, -1, 0) {
                        builder.add_face(offset, crate::mesh::Direction::Down, uv);
                    }
                    if x == 15 || is_transparent(1, 0, 0) {
                        builder.add_face(offset, crate::mesh::Direction::West, uv);
                    }
                    if x == 0 || is_transparent(-1, 0, 0) {
                        builder.add_face(offset, crate::mesh::Direction::East, uv);
                    }
                }
            }
        }

        let material = atlas.material.clone();

        let mesh = builder.build(material, device);
        self.cache.insert(chunk.pos, Arc::new(mesh));
        chunk.dirty.store(false, std::sync::atomic::Ordering::Relaxed);
        self.cache.get(&chunk.pos).unwrap().clone()
    }
}
