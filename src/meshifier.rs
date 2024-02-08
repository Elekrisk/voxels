use std::{cell::RefCell, sync::Arc};

use cgmath::{EuclideanSpace, Point2, Point3, Vector2, Vector3, Zero};
use wgpu::naga::FastHashMap;

use crate::{
    game::{
        atlas::Atlas,
        block::{BlockId, BlockRegistry},
        chunk::{Chunk, ChunkPos, ChunkRelativeBlockPos},
        world::World,
    },
    mesh::{Direction, Mesh, MeshBuilder, MeshVertex},
};

pub struct ChunkMeshifier {
    cache: FastHashMap<ChunkPos, Arc<Mesh>>,
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
        if !chunk.get_dirty() && self.cache.contains_key(&chunk.pos) {
            return self.cache.get(&chunk.pos).unwrap().clone();
        }

        let mut builder = MeshBuilder::new();

        let neighbouring_chunks = Direction::ALL.map(|d| world.chunk(chunk.pos + d.normal()));

        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let position = Point3::new(x, y, z).into();
                    let block = &chunk.block(position);
                    let attr = block_registry.get(block.id).unwrap();
                    if attr.invisible {
                        continue;
                    }

                    let offset = Vector3::new(x as _, y as _, z as _) + Vector3::new(0.5, 0.5, 0.5);

                    let is_transparent = |[dx, dy, dz]: [isize; 3]| {
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

                    for (dir, neighbour_chunk) in
                        Direction::ALL.into_iter().zip(&neighbouring_chunks)
                    {
                        let build = if dir.axle().of(position) == dir.chunk_limit() {
                            if let Some(chunk) = neighbour_chunk {
                                let mut block_pos = position;
                                dir.axle().set(&mut block_pos, dir.inverse().chunk_limit());
                                let block = chunk.block(block_pos);
                                block_registry.get(block.id).unwrap().transparent
                            } else {
                                true
                            }
                        } else {
                            is_transparent(dir.normal().into())
                        };

                        if build {
                            self.build_face(
                                &mut builder,
                                offset,
                                chunk,
                                world,
                                position,
                                dir,
                                uv,
                                block_registry,
                            );
                        }
                    }
                }
            }
        }

        let material = atlas.material.clone();

        let mesh = builder.build(material, device);
        self.cache.insert(chunk.pos, Arc::new(mesh));
        chunk.set_dirty(false);
        self.cache.get(&chunk.pos).unwrap().clone()
    }

    fn build_face(
        &mut self,
        builder: &mut MeshBuilder,
        offset: Vector3<f32>,
        chunk: &Chunk,
        world: &World,
        position: ChunkRelativeBlockPos,
        direction: Direction,
        uv: [Point2<f32>; 4],
        block_registry: &BlockRegistry,
    ) {
        let no = 0.0 / 6.0;
        let li = 1.0 / 6.0;
        let me = 2.0 / 6.0;
        let he = 3.0 / 6.0;

        let pos = Point3::from(position).cast::<isize>().unwrap();

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
            let pos: Point3<isize> =
                pos + direction.on_plane(offset.into()).to_vec() + direction.normal();
            if pos.x < 0 || pos.y < 0 || pos.z < 0 || pos.x > 15 || pos.y > 15 || pos.z > 15 {
                let mut chunk_pos = chunk.pos;
                let mut block_pos = pos;
                for dir in Direction::ALL.into_iter() {
                    if dir.chunk_limit() == 0 && dir.axle().of(pos.to_vec()) < 0 {
                        chunk_pos += dir.normal();
                        dir.axle().set(&mut block_pos, 15);
                    } else if dir.chunk_limit() == 15 && dir.axle().of(pos.to_vec()) > 15 {
                        chunk_pos += dir.normal();
                        dir.axle().set(&mut block_pos, 0);
                    }
                }

                if let Some(chunk) = world.chunk(chunk_pos) {
                    if !block_registry
                        .get(chunk.block(block_pos.cast().unwrap().into()).id)
                        .unwrap()
                        .transparent
                    {
                        blocking |= 0x80;
                    }
                }

                continue;
            }
            let pos = pos.cast().unwrap();
            if !block_registry
                .get(chunk.block(pos.into()).id)
                .unwrap()
                .transparent
            {
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
            .map(|p| direction.on_plane(p.into()) + direction.normal() * 0.5)
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
