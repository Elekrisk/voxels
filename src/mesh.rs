use std::{ops::Range, sync::Arc};

use wgpu::util::DeviceExt;

use crate::texture::Texture;

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex for MeshVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    material: Arc<Material>,
}

impl Mesh {
    pub fn new(
        vertices: &[MeshVertex],
        indices: &[u32],
        material: Arc<Material>,
        device: &wgpu::Device,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", "TEMP!! ")),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", "TEMP!! ")),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_elements: indices.len() as u32,
            material,
        }
    }
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    pub fn new(
        image: &[u8],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let diffuse_texture = Texture::from_bytes(device, queue, image, "TEMP!! ").unwrap();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: None,
        });

        Material {
            name: "TEMP !!".into(),
            diffuse_texture,
            bind_group,
        }
    }
}

pub trait DrawModel<'a> {
    fn draw_mesh(&mut self, mesh: &'a Mesh, camera_bind_group: &'a wgpu::BindGroup);
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh, camera_bind_group: &'a wgpu::BindGroup) {
        self.draw_mesh_instanced(mesh, 0..1, camera_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &mesh.material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}

use cgmath::{ElementWise, Vector2, Vector3};

pub struct MeshBuilder {
    vertices: Vec<MeshVertex>,
    indices: Vec<u32>,
}

pub enum Direction {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self {
            vertices: vec![],
            indices: vec![],
        }
    }

    pub fn add_face(&mut self, offset: Vector3<f32>, direction: Direction) {
        let pos = match direction {
            Direction::North => [
                [-0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, -0.5, 0.5],
                [-0.5, -0.5, 0.5],
            ],
            Direction::East => [
                [-0.5, 0.5, -0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, -0.5, -0.5],
            ],
            Direction::South => [
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
            ],
            Direction::West => [
                [0.5, 0.5, 0.5],
                [0.5, 0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
            ],
            Direction::Up => [
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
            ],
            Direction::Down => [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
            ],
        };

        let indices = [0, 3, 1, 1, 3, 2];
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

        let normals = match direction {
            Direction::North => [[0.0, 0.0, 1.0]; 4],
            Direction::East => [[-1.0, 0.0, 0.0]; 4],
            Direction::South => [[0.0, 0.0, -1.0]; 4],
            Direction::West => [[1.0, 0.0, 0.0]; 4],
            Direction::Up => [[0.0, 1.0, 0.0]; 4],
            Direction::Down => [[0.0, -1.0, 0.0]; 4],
        };

        let vertices = pos
            .into_iter()
            .zip(uv)
            .zip(normals)
            .map(|((pos, uv), norm)| MeshVertex {
                position: [pos[0] + offset.x, pos[1] + offset.y, pos[2] + offset.z],
                tex_coords: uv,
                normal: norm,
            });

        let offset = self.vertices.len() as u32;
        self.vertices.extend(vertices);
        self.indices.extend_from_slice(&indices.map(|x| x + offset));
    }

    pub fn build(self, material: Arc<Material>, device: &wgpu::Device) -> Mesh {
        Mesh::new(&self.vertices, &self.indices, material, device)
    }
}
