use std::sync::Arc;

use cgmath::Point2;

use crate::{mesh::Material, texture::Texture};

pub struct Atlas {
    pub material: Arc<Material>,
    pub cell_size: usize,
}

impl Atlas {
    pub fn new(material: Arc<Material>, cell_size: usize) -> Self {
        Self {
            material,
            cell_size,
        }
    }

    pub fn uv(&self, coords: impl Into<Point2<usize>>) -> [Point2<f32>; 4] {
        let w = self.material.diffuse_texture.texture.width() as usize;
        let h = self.material.diffuse_texture.texture.height() as usize;

        let cw = w / self.cell_size;
        let ch = h / self.cell_size;

        let step_x = 1.0 / cw as f32;
        let step_y = 1.0 / ch as f32;

        let coords: Point2<usize> = coords.into();

        let top_left = Point2::new(coords.x as f32 * step_x, coords.y as f32 * step_y);
        let top_right = Point2::new((coords.x + 1) as f32 * step_x, coords.y as f32 * step_y);
        let bottom_left = Point2::new(coords.x as f32 * step_x, (coords.y + 1) as f32 * step_y);
        let bottom_right = Point2::new(
            (coords.x + 1) as f32 * step_x,
            (coords.y + 1) as f32 * step_y,
        );

        [top_left, top_right, bottom_right, bottom_left]
    }
}
