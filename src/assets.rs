use std::{collections::HashMap, sync::Arc};

use crate::mesh::Material;

pub struct AssetManager {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    layout: Arc<wgpu::BindGroupLayout>,
    materials: HashMap<String, Arc<Material>>,
}

impl AssetManager {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        layout: Arc<wgpu::BindGroupLayout>,
    ) -> Self {
        Self {
            device,
            queue,
            layout,
            materials: HashMap::new(),
        }
    }

    pub fn load_material(&mut self, path: impl Into<String>) -> Option<Arc<Material>> {
        let path = path.into();
        let data = std::fs::read(&path).ok()?;
        let material = Material::new(&data, &self.device, &self.queue, &self.layout);
        let material = Arc::new(material);
        self.materials.insert(path, material.clone());
        Some(material)
    }
}
