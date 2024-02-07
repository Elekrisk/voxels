use bevy_ecs::{
    component::Component,
    system::{Query, Res},
};
use cgmath::{EuclideanSpace, InnerSpace, MetricSpace, Point3, Vector3, Zero};

use super::{block::BlockRegistry, world::World, DeltaTime, Position, Velocity};

#[derive(Clone, Copy, PartialEq, Component)]
pub struct Collider {
    pub enabled: bool,
    pub gravity: bool,
    pub extents: Vector3<f32>,
}

const GRAVITY: Vector3<f32> = Vector3::new(0.0, -30.0, 0.0);

pub fn physics_system(
    mut query: Query<(&mut Position, &mut Velocity, &Collider)>,
    dt: Res<DeltaTime>,
    world: Res<World>,
    block_registry: Res<BlockRegistry>,
) {
    for (mut pos, mut vel, col) in &mut query {
        let prev_pos = pos.0;
        pos.0 += vel.0 * dt.0;
        if col.gravity {
            vel.0 += GRAVITY * dt.0;
        }

        if !col.enabled {
            continue;
        }

        let min = Point3::new(
            pos.0.x - col.extents.x / 2.0,
            pos.0.y,
            pos.0.z - col.extents.z / 2.0,
        );
        let max = Point3::new(
            pos.0.x + col.extents.x / 2.0,
            pos.0.y + col.extents.y,
            pos.0.z + col.extents.z / 2.0,
        );

        let min_block_pos = min.cast::<isize>().unwrap();
        let max_block_pos = max.cast::<isize>().unwrap();

        let mut collisions = vec![];

        for block_pos_x in min_block_pos.x..=max_block_pos.x {
            for block_pos_y in min_block_pos.y..=max_block_pos.y {
                for block_pos_z in min_block_pos.z..=max_block_pos.z {
                    let block_pos = Point3::new(block_pos_x, block_pos_y, block_pos_z);

                    let chunk_pos = block_pos.map(|e| e.div_euclid(16));
                    let rel_pos = block_pos.map(|e| e.rem_euclid(16) as _);

                    let Some(chunk) = world.chunk(chunk_pos) else {
                        continue;
                    };
                    let block = chunk.block(rel_pos);
                    let attrs = block_registry.get(block.id).unwrap();
                    if attrs.invisible {
                        continue;
                    }

                    collisions.push((
                        block_pos,
                        (block_pos.cast::<f32>().unwrap() + Vector3::new(0.5, 0.5, 0.5))
                            .distance2(min.midpoint(max)),
                    ));
                }
            }
        }

        collisions.sort_by(|(_, adist), (_, bdist)| adist.partial_cmp(bdist).unwrap());

        for (collision, _) in collisions {
            let block_min_extended = collision.cast::<f32>().unwrap()
                - Vector3::new(col.extents.x / 2.0, col.extents.y, col.extents.z / 2.0);
            let block_max_extended = collision.cast::<f32>().unwrap()
                + Vector3::new(col.extents.x / 2.0 + 1.0, 1.0, col.extents.z / 2.0 + 1.0);

            let overlap = Vector3::new(
                (block_max_extended.x - pos.0.x).min(pos.0.x - block_min_extended.x),
                (block_max_extended.y - pos.0.y).min(pos.0.y - block_min_extended.y),
                (block_max_extended.z - pos.0.z).min(pos.0.z - block_min_extended.z),
            );

            if overlap.x <= 0.0 || overlap.y <= 0.0 || overlap.z <= 0.0 {
                continue;
            }

            let dir = pos.0 - prev_pos;
            if dir.is_zero() {
                continue;
            }
            let dir = dir.normalize();

            let scaled_dir = Vector3::new(dir.x / overlap.x, dir.y / overlap.y, dir.z / overlap.z);

            // println!("{overlap:?}");
            // println!("{dir:?}");
            // println!("{scaled_dir:?}");

            let diff = pos.0 - block_min_extended.midpoint(block_max_extended);
            // println!("{diff:?}");

            if scaled_dir.x.abs() >= scaled_dir.y.abs().max(scaled_dir.z.abs()) {
                pos.0.x -= overlap.x * scaled_dir.x.signum();
                if diff.x.signum() != vel.0.x.signum() {
                    vel.0.x = 0.0;
                }
            } else if scaled_dir.y.abs() >= scaled_dir.x.abs().max(scaled_dir.z.abs()) {
                pos.0.y -= overlap.y * scaled_dir.y.signum();
                if diff.y.signum() != vel.0.y.signum() {
                    vel.0.y = 0.0;
                }
            } else if scaled_dir.z.abs() >= scaled_dir.x.abs().max(scaled_dir.y.abs()) {
                pos.0.z -= overlap.z * scaled_dir.z.signum();
                if diff.z.signum() != vel.0.z.signum() {
                    vel.0.z = 0.0;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Box {
    min: Point3<f32>,
    max: Point3<f32>,
}

pub fn jump_height_to_vel(height: f32) -> f32 {
    f32::sqrt(-2.0 * GRAVITY.y * height)
}
