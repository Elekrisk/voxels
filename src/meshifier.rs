use std::{cell::RefCell, sync::Arc};

use cgmath::{EuclideanSpace, Point2, Point3, Vector2, Vector3, Zero};
use wgpu::naga::FastHashMap;

use crate::{
    game::{
        atlas::Atlas,
        block::{BlockId, BlockRegistry},
        chunk::Chunk,
        world::World,
    },
    mesh::{Direction, Mesh, MeshBuilder, MeshVertex},
};

pub struct ChunkMeshifier {
    cache: FastHashMap<Point3<isize>, Arc<Mesh>>,
    pub enable_ao: bool,
}

impl ChunkMeshifier {
    pub fn new() -> Self {
        Self {
            cache: FastHashMap::default(),
            enable_ao: true,
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
        println!("Meshifying chunk {},{},{}", chunk.pos.x, chunk.pos.y, chunk.pos.z);
        if !chunk.dirty.load(std::sync::atomic::Ordering::Relaxed)
            && self.cache.contains_key(&chunk.pos)
        {
            return self.cache.get(&chunk.pos).unwrap().clone();
        }

        let mut builder = MeshBuilder::new();

        let north_chunk = world.chunk(chunk.pos + Direction::North.normal());
        let south_chunk = world.chunk(chunk.pos + Direction::South.normal());
        let east_chunk = world.chunk(chunk.pos + Direction::East.normal());
        let west_chunk = world.chunk(chunk.pos + Direction::West.normal());
        let up_chunk = world.chunk(chunk.pos + Direction::Up.normal());
        let down_chunk = world.chunk(chunk.pos + Direction::Down.normal());

        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let position = Point3::new(x, y, z);
                    let block = &chunk.block(position);
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


                    let build_north = if z == 15 {
                        if let Some(chunk) = north_chunk {
                            let block = chunk.block([x, y, 0]);
                            block_registry.get(block.id).unwrap().transparent
                        } else {
                            true
                        }
                    } else {
                        is_transparent(0, 0, 1)
                    };

                    let build_south = if z == 0 {
                        if let Some(chunk) = south_chunk {
                            let block = chunk.block([x, y, 15]);
                            block_registry.get(block.id).unwrap().transparent
                        } else {
                            true
                        }
                    } else {
                        is_transparent(0, 0, -1)
                    };

                    let build_east = if x == 0 {
                        if let Some(chunk) = east_chunk {
                            let block = chunk.block([15, y, z]);
                            block_registry.get(block.id).unwrap().transparent
                        } else {
                            true
                        }
                    } else {
                        is_transparent(-1, 0, 0)
                    };

                    let build_west = if x == 15 {
                        if let Some(chunk) = west_chunk {
                            let block = chunk.block([0, y, z]);
                            block_registry.get(block.id).unwrap().transparent
                        } else {
                            true
                        }
                    } else {
                        is_transparent(1, 0, 0)
                    };

                    let build_up = if y == 15 {
                        if let Some(chunk) = up_chunk {
                            let block = chunk.block([x, 0, z]);
                            block_registry.get(block.id).unwrap().transparent
                        } else {
                            true
                        }
                    } else {
                        is_transparent(0, 1, 0)
                    };

                    let build_down = if y == 0 {
                        if let Some(chunk) = south_chunk {
                            let block = chunk.block([x, 15, z]);
                            block_registry.get(block.id).unwrap().transparent
                        } else {
                            true
                        }
                    } else {
                        is_transparent(0, -1, 0)
                    };

                    if build_north {
                        // builder.add_face(offset, crate::mesh::Direction::North, uv);
                        self.build_face(
                            &mut builder,
                            offset,
                            chunk,
                            position,
                            Direction::North,
                            uv,
                            block_registry,
                        );
                    }
                    if build_south {
                        // builder.add_face(offset, crate::mesh::Direction::South, uv);
                        self.build_face(
                            &mut builder,
                            offset,
                            chunk,
                            position,
                            Direction::South,
                            uv,
                            block_registry,
                        );
                    }
                    if build_up {
                        // builder.add_face(offset, crate::mesh::Direction::Up, uv);
                        self.build_face(
                            &mut builder,
                            offset,
                            chunk,
                            position,
                            Direction::Up,
                            uv,
                            block_registry,
                        );
                    }
                    if build_down {
                        // builder.add_face(offset, crate::mesh::Direction::Down, uv);
                        self.build_face(
                            &mut builder,
                            offset,
                            chunk,
                            position,
                            Direction::Down,
                            uv,
                            block_registry,
                        );
                    }
                    if build_west {
                        // builder.add_face(offset, crate::mesh::Direction::West, uv);
                        self.build_face(
                            &mut builder,
                            offset,
                            chunk,
                            position,
                            Direction::West,
                            uv,
                            block_registry,
                        );
                    }
                    if build_east {
                        // builder.add_face(offset, crate::mesh::Direction::East, uv);
                        self.build_face(
                            &mut builder,
                            offset,
                            chunk,
                            position,
                            Direction::East,
                            uv,
                            block_registry,
                        );
                    }
                }
            }
        }

        let material = atlas.material.clone();

        let mesh = builder.build(material, device);
        self.cache.insert(chunk.pos, Arc::new(mesh));
        chunk
            .dirty
            .store(false, std::sync::atomic::Ordering::Relaxed);
        self.cache.get(&chunk.pos).unwrap().clone()
    }

    fn build_face(
        &mut self,
        builder: &mut MeshBuilder,
        offset: Vector3<f32>,
        chunk: &Chunk,
        position: Point3<usize>,
        direction: Direction,
        uv: [Point2<f32>; 4],
        block_registry: &BlockRegistry,
    ) {
        let no = 0.0 / 6.0;
        let li = 1.0 / 6.0;
        let me = 2.0 / 6.0;
        let he = 3.0 / 6.0;

        let pos = position.cast::<isize>().unwrap();

        let block_offsets = [
            [-1, 0],
            [-1, 1],
            [0, 1],
            [1, 1],
            [1, 0],
            [1, -1],
            [0, -1],
            [-1, -1],
        ];

        let mut blocking: u8 = 0;

        for offset in block_offsets {
            blocking >>= 1;
            let pos: Point3<isize> = pos
                + direction.on_plane(offset.into()).to_vec()
                + direction.normal();
            if pos.x < 0 || pos.y < 0 || pos.z < 0 || pos.x > 15 || pos.y > 15 || pos.z > 15 {
                continue;
            }
            let pos = pos.cast().unwrap();
            if !block_registry.get(chunk.block(pos).id).unwrap().transparent {
                blocking |= 0x80;
            }
        }

        let get_ao = |blocks: u8| match blocks & 0b111 {
            0b000 => no,
            0b001 => li,
            0b010 => li,
            0b011 => me,
            0b100 => li,
            0b101 => he,
            0b110 => me,
            0b111 => he,
            _ => unreachable!(),
        };

        let tl_ao = get_ao(blocking);
        let tr_ao = get_ao(blocking >> 2);
        let br_ao = get_ao(blocking >> 4);
        let bl_ao = get_ao(blocking >> 6 | blocking << 2);

        let vertex_positions = [[-0.5, 0.5], [0.5, 0.5], [0.5, -0.5], [-0.5, -0.5]]
            .into_iter()
            .map(|p|  direction.on_plane(p.into()) + direction.normal() * 0.5)
            .collect::<Vec<_>>();
        let vertex_uvs = uv;
        let vertex_aos = [tl_ao, tr_ao, br_ao, bl_ao];
        let vertex_indices = [0, 3, 1, 1, 3, 2];

        let vertices = (0..4)
            .map(|i| MeshVertex {
                position: (vertex_positions[i] + offset).into(),
                tex_coords: vertex_uvs[i].into(),
                ambient_occlusion: if self.enable_ao { vertex_aos[i] } else { 0.0 },
                normal: direction.normal().into(),
            })
            .collect::<Vec<_>>();

        builder.add_vert_indices(&vertices, &vertex_indices);
    }
}
