use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::{mesh::Mesh, Instance};


pub struct Object {
    pub mesh: Arc<Mesh>,
    instance: Instance,
    instance_dirty: bool,
    pub instance_buffer: wgpu::Buffer,
}

impl Object {
    pub fn new(mesh: Arc<Mesh>, instance: Instance, device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::bytes_of(&instance.to_raw()),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            mesh,
            instance,
            instance_dirty: false,
            instance_buffer: buffer
        }
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn edit_instance(&mut self, f: impl FnOnce(&mut Instance)) {
        let orig = self.instance.clone();
        f(&mut self.instance);
        if orig != self.instance {
            self.instance_dirty = true;
        }
    }

    pub fn update_instance_buffer(&mut self, queue: &wgpu::Queue) {
        if self.instance_dirty {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::bytes_of(&self.instance.to_raw()));
            self.instance_dirty = false;
        }
    }
}
