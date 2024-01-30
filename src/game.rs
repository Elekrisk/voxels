use wgpu::RenderPass;

use crate::{
    assets::AssetManager,
    mesh::{DrawModel, Mesh},
    meshifier::Meshifier,
};

use self::{
    block::{BlockAttributes, BlockId, BlockRegistry},
    world::World,
};

pub mod block;
pub mod chunk;
pub mod world;

pub struct Game {
    pub world: World,
    pub block_registry: BlockRegistry,
    mesh: Option<&'static Mesh>,
}

impl Game {
    pub fn new(asset_manager: &mut AssetManager) -> Self {
        let mut block_registry = BlockRegistry::new();

        let air_block_attr = BlockAttributes {
            transparent: true,
            invisible: true,
            material: asset_manager.load_material("assets/image.png").unwrap(),
        };
        block_registry.register(BlockId(0), air_block_attr);

        let dirt_block_attr = BlockAttributes {
            transparent: false,
            invisible: false,
            material: asset_manager.load_material("assets/image.png").unwrap(),
        };
        block_registry.register(BlockId(1), dirt_block_attr);

        Self {
            world: World::new(),
            block_registry,
            mesh: None,
        }
    }

    pub fn render<'a, 'b>(
        &mut self,
        render_pass: &mut RenderPass<'a>,
        device: &wgpu::Device,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        if self.mesh.is_none() {
            let mesh = Meshifier::meshify(&self.world.chunks[0], &self.block_registry, device);
            self.mesh = Some(Box::leak(Box::new(mesh)));
        }
        render_pass.draw_mesh_instanced(&self.mesh.as_ref().unwrap(), 0..1, camera_bind_group);
    }
}
