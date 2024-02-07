use super::{
    block::{Block, BlockId, BlockMetadata, BlockRegistry},
    chunk::Chunk,
};
use bevy_ecs::system::Resource;
use cgmath::{EuclideanSpace, Point3, Vector3};
use rand::Rng;
use wgpu::naga::FastHashMap;

#[derive(Resource)]
pub struct World {
    pub chunks: FastHashMap<Point3<isize>, Chunk>,
}

impl World {
    pub fn new() -> Self {
        Self { chunks: FastHashMap::default() }
    }

    pub fn chunk(&self, pos: impl Into<Point3<isize>>) -> Option<&Chunk> {
        self.chunks.get(&pos.into())
    }

    pub fn chunk_mut(&mut self, pos: impl Into<Point3<isize>>) -> Option<&mut Chunk> {
        self.chunks.get_mut(&pos.into())
    }

    pub fn generate_chunk(&mut self, pos: impl Into<Point3<isize>>) {
        let mut chunk = Chunk::new(pos.into());

        for (_, block) in chunk.block_iter_mut() {
            block.id.0 = rand::thread_rng().gen::<u8>() % 5;
        }

        self.chunks.insert(chunk.pos, chunk);
    }

    pub fn create_empty_chunk(&mut self, pos: impl Into<Point3<isize>>) {
        let pos = pos.into();
        self.chunks.insert(pos, Chunk::new(pos));
    }

    pub fn delete_chunk(&mut self, pos: impl Into<Point3<isize>>) {
        self.chunks.remove(&pos.into());
    }

    pub fn place_block(&mut self, block: Block, pos: impl Into<Point3<isize>>) {
        let pos = pos.into();
        let chunk_pos = pos.map(|e| e.div_euclid(16));
        let rel_pos = pos.map(|e| e.rem_euclid(16) as _);
        let chunk = if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk
        } else {
            self.create_empty_chunk(chunk_pos);
            self.chunks.get_mut(&chunk_pos).unwrap()
        };

        *chunk.block_mut(rel_pos) = block;
        chunk.dirty.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn raycast(
        &self,
        origin: Point3<f32>,
        dir: Vector3<f32>,
        range: f32,
        block_registry: &BlockRegistry,
    ) -> Option<HitInfo> {
        let mut chunk_pos = origin.cast::<isize>().unwrap().map(|e| e.div_euclid(16));
        let mut rel_origin = origin - chunk_pos.cast::<f32>().unwrap().to_vec() * 16.0;

        let mut remaining_range = range;

        fn in_range_f32(v: f32) -> bool {
            v >= -0.001 && v <= 16.001
        }

        fn in_range(v: Point3<f32>) -> bool {
            in_range_f32(v.x) && in_range_f32(v.y) && in_range_f32(v.z)
        }

        fn fix_range_inside(mut v: Point3<f32>) -> Point3<f32> {
            if v.x < 0.0 {
                v.x = 0.0;
            } else if v.x >= 16.0 {
                v.x = 15.999;
            }
            if v.y < 0.0 {
                v.y = 0.0;
            } else if v.y >= 16.0 {
                v.y = 15.999;
            }
            if v.z < 0.0 {
                v.z = 0.0;
            } else if v.z >= 16.0 {
                v.z = 15.999;
            }

            v
        }

        fn fix_range_on_side(mut v: Point3<f32>) -> Point3<f32> {
            if v.x < 0.0 {
                v.x = 0.0;
            } else if v.x > 16.0 {
                v.x = 16.0;
            }
            if v.y < 0.0 {
                v.y = 0.0;
            } else if v.y > 16.0 {
                v.y = 16.0;
            }
            if v.z < 0.0 {
                v.z = 0.0;
            } else if v.z > 16.0 {
                v.z = 16.0;
            }

            v
        }

        loop {
            let chunk = self.chunks.get(&chunk_pos);

            let mut block_hits = vec![];

            if let Some(chunk) = chunk {

                // X
                for x in 0..=16 {
                    let t = (x as f32 - rel_origin.x) / dir.x;
                    if t < -0.001 || t > remaining_range {
                        continue;
                    }
                    let p = rel_origin + dir * t + Vector3::unit_x() * 0.001 * dir.x.signum();

                    if in_range(p) {
                        let pos = fix_range_inside(p).cast::<usize>().unwrap();
                        let block = chunk.block(pos);
                        let attr = block_registry.get(block.id).unwrap();
                        if !attr.invisible {
                            let position = chunk_pos.cast::<isize>().unwrap() * 16 + pos.cast::<isize>().unwrap().to_vec();
                            let normal = Vector3::unit_x() * -dir.x.signum();

                            let hitinfo = HitInfo {
                                position,
                                normal,
                            };

                            block_hits
                                .push((hitinfo, t));
                        }
                    }
                }

                // Y
                for y in 0..=16 {
                    let t = (y as f32 - rel_origin.y) / dir.y;
                    if t < -0.001 || t > remaining_range {
                        continue;
                    }
                    let p = rel_origin + dir * t + Vector3::unit_y() * 0.001 * dir.y.signum();

                    if in_range(p) {
                        let pos = fix_range_inside(p).cast::<usize>().unwrap();
                        let block = chunk.block(pos);
                        let attr = block_registry.get(block.id).unwrap();
                        if !attr.invisible {
                            let position = chunk_pos.cast::<isize>().unwrap() * 16 + pos.cast::<isize>().unwrap().to_vec();
                            let normal = Vector3::unit_y() * -dir.y.signum();

                            let hitinfo = HitInfo {
                                position,
                                normal,
                            };

                            block_hits
                                .push((hitinfo, t));
                        }
                    }
                }

                // Z
                for z in 0..=16 {
                    let t = (z as f32 - rel_origin.z) / dir.z;
                    if t < -0.001 || t > remaining_range {
                        continue;
                    }
                    let p = rel_origin + dir * t + Vector3::unit_z() * 0.001 * dir.z.signum();

                    if in_range(p) {
                        let pos = fix_range_inside(p).cast::<usize>().unwrap();
                        let block = chunk.block(pos);
                        let attr = block_registry.get(block.id).unwrap();
                        if !attr.invisible {
                            let position = chunk_pos.cast::<isize>().unwrap() * 16 + pos.cast::<isize>().unwrap().to_vec();
                            let normal = Vector3::unit_z() * -dir.z.signum();

                            let hitinfo = HitInfo {
                                position,
                                normal,
                            };

                            block_hits
                                .push((hitinfo, t));
                        }
                    }
                }

                if !block_hits.is_empty() {
                    block_hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    return Some(block_hits[0].0);
                }
            }

 
            let next_x = if dir.x >= 0.0 { 16.0 } else { 0.0 };

            let xt = (next_x - rel_origin.x) / dir.x;
            let xp = rel_origin + dir * xt;

            if xt <= remaining_range && in_range(xp) {
                chunk_pos.x += dir.x.signum() as isize;
                rel_origin = fix_range_on_side(xp + Vector3::unit_x() * 16.0 * -dir.x.signum());
                remaining_range -= xt;
                continue;
            }

            let next_y = if dir.y >= 0.0 { 16.0 } else { 0.0 };

            let yt = (next_y - rel_origin.y) / dir.y;
            let yp = rel_origin + dir * yt;

            if yt <= remaining_range && in_range(yp) {
                chunk_pos.y += dir.y.signum() as isize;
                rel_origin = fix_range_on_side(yp + Vector3::unit_y() * 16.0 * -dir.y.signum());
                remaining_range -= yt;
                continue;
            }

            let next_z = if dir.z >= 0.0 { 16.0 } else { 0.0 };

            let zt = (next_z - rel_origin.z) / dir.z;
            let zp = rel_origin + dir * zt;

            if zt <= remaining_range && in_range(zp) {
                chunk_pos.z += dir.z.signum() as isize;
                rel_origin = fix_range_on_side(zp + Vector3::unit_z() * 16.0 * -dir.z.signum());
                remaining_range -= zt;
                continue;
            }


            break;
        }

        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HitInfo {
    pub position: Point3<isize>,
    pub normal: Vector3<f32>,
}

pub fn to_block_pos(pos: Point3<f32>) -> Point3<isize> {
    pos.cast().unwrap()
}

pub fn to_chunk_rel_pos(pos: Point3<isize>) -> (Point3<isize>, Point3<usize>) {
    let chunk_pos = pos.map(|e| e.div_euclid(16));
    let rel_pos = pos.map(|e| e.rem_euclid(16) as usize);
    (chunk_pos, rel_pos)
}
