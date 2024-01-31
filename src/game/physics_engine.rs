use std::time::Duration;

use cgmath::{EuclideanSpace, Point3, Vector3, Zero};

use super::{entity::Entity, world::World};


pub struct PhysicsEngine {

}

impl PhysicsEngine {
    const GRAVITY: Vector3<f32> = Vector3::new(0.0, -10.0, 0.0);

    pub fn new() -> Self {
        Self {

        }
    }

    pub fn do_physics(&self, world: &World, entities: &mut [Entity], dt: Duration) {
        let dt = dt.as_secs_f32();
        for entity in entities {
            entity.pos += entity.vel * dt;
            entity.vel += Self::GRAVITY * dt;

            let chunk_pos = entity.pos.map(|e| f32::floor(e / 16.0)).cast::<isize>().unwrap();

            let Some(chunk) = world.chunks.iter().find(|c| c.pos == chunk_pos) else {
                continue;
            };

            let chunk_local_pos = entity.pos - chunk.pos.cast::<f32>().unwrap() * 16.0;
            let chunk_local_pos = Point3::from_vec(chunk_local_pos);
            

            // println!("{:?}", chunk_local_pos);

            let block_pos = chunk_local_pos.cast::<usize>().unwrap();

            if chunk.block(block_pos).id.0 != 0 {
                let block_center = block_pos.cast::<f32>().unwrap() + Vector3::<f32>::new(0.5, 0.5, 0.5);
                let diff = chunk_local_pos - block_center;
                if diff.x.abs() > diff.y.abs() && diff.x.abs() > diff.z.abs() {
                    entity.vel.x = 0.0;
                    entity.pos.x = block_center.x + 0.5 * diff.x.signum();
                }
                if diff.z.abs() > diff.x.abs() && diff.z.abs() > diff.y.abs() {
                    entity.vel.z = 0.0;
                    entity.pos.z = block_center.z + 0.5 * diff.z.signum();
                }
                if diff.y.abs() > diff.x.abs() && diff.y.abs() > diff.z.abs() {
                    entity.vel.y = 0.0;
                    entity.pos.y = block_center.y + 0.5 * diff.y.signum();
                }
            }
        }
    }
}