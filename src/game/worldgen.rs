use cgmath::{Point3, Vector2};
use noise::{BasicMulti, NoiseFn, OpenSimplex, Perlin, Simplex};
use rand::{thread_rng, Rng};

use super::{block::{Block, BlockId, BlockMetadata}, chunk::{Chunk, ChunkPos, ChunkRelativeBlockPos}};

type Noise = impl NoiseFn<f64, 2>;

pub struct Worldgen {
    elevation_noise: Noise,
    noise_offset: Vector2<f64>,
}

impl Worldgen {
    pub fn new() -> Self {
        let base_elevation = OpenSimplex::new(thread_rng().gen());
        let noise = base_elevation;
        Self {
            elevation_noise: noise,
            noise_offset: [thread_rng().gen_range(-1000.0..=1000.0), thread_rng().gen_range(-1000.0..=1000.0)].into()
        }
    }

    pub fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        let offset = Point3::from(pos + ChunkRelativeBlockPos::from(Point3::new(0, 0, 0)));

        let mut chunk = Chunk::new(pos);

        for x in 0..Chunk::SIZE as isize {
            for z in 0..Chunk::SIZE as isize {
                let global_x = offset.x + x;
                let global_z = offset.z + z;
                let value = self
                    .elevation_noise
                    .get([global_x as f64 / 16.0 + self.noise_offset.x, global_z as f64 / 16.0 + self.noise_offset.y])
                    * 8.0;

                let max_height = value;

                let dirt_height = value - 3.0;

                for y in 0..Chunk::SIZE as isize {
                    let global_y = offset.y + y;
                    let id = if global_y as f64 > max_height {
                        if (global_y as f64) < 0.0 {
                            4
                        } else {
                            0
                        }
                    } else if global_y as f64 > dirt_height {
                        1
                    } else {
                        2
                    };

                    chunk
                        .block_mut(Point3::new(x, y, z).cast().unwrap().into())
                        .id
                        .0 = id;
                    // chunk.place_block(Point3::new(x, y, z).cast().unwrap().into(), Block {
                    //     id: BlockId(id),
                    //     metadata: BlockMetadata(0)
                    // });
                }
            }
        }

        chunk
    }
}
