use crate::renderer::{Vertex, TEXTURE_FORMAT};
use ultraviolet::Vec2;
use wgpu::util::DeviceExt;

pub struct Assets {
    pub surface_model: Model,
    pub bullet_model: Model,
    pub mouse_model: Model,
    pub mouse_helmet_model: Model,
    pub torus_model: Model,

    pub texture_bind_group_layout: wgpu::BindGroupLayout,

    pub surface_texture: wgpu::BindGroup,
    pub colours_texture: wgpu::BindGroup,
    pub hud_texture: wgpu::BindGroup,
    pub mouse_texture: wgpu::BindGroup,
}

impl Assets {
    pub fn new(device: &wgpu::Device) -> anyhow::Result<(Self, wgpu::CommandBuffer)> {
        let mut init_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Cheese init_encoder"),
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Cheese texture bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                    },
                    count: None,
                }],
            });

        let assets = Self {
            surface_model: Model::load(include_bytes!("../models/surface.obj"), device)?,
            bullet_model: Model::load(include_bytes!("../models/bullet.obj"), device)?,
            mouse_model: Model::load(include_bytes!("../models/mouse.obj"), device)?,
            mouse_helmet_model: Model::load(include_bytes!("../models/mouse_helmet.obj"), device)?,
            torus_model: Model::load(include_bytes!("../models/torus.obj"), device)?,

            surface_texture: load_texture(
                include_bytes!("../textures/surface.png"),
                &texture_bind_group_layout,
                device,
                &mut init_encoder,
            )?,
            colours_texture: load_texture(
                include_bytes!("../textures/colours.png"),
                &texture_bind_group_layout,
                device,
                &mut init_encoder,
            )?,
            hud_texture: load_texture(
                include_bytes!("../textures/hud.png"),
                &texture_bind_group_layout,
                device,
                &mut init_encoder,
            )?,
            mouse_texture: load_texture(
                include_bytes!("../textures/mouse.png"),
                &texture_bind_group_layout,
                device,
                &mut init_encoder,
            )?,

            texture_bind_group_layout,
        };

        Ok((assets, init_encoder.finish()))
    }
}

pub struct Model {
    pub buffer: wgpu::Buffer,
    pub num_vertices: u32,
}

impl Model {
    pub fn load(obj_bytes: &[u8], device: &wgpu::Device) -> anyhow::Result<Self> {
        let mut reader = std::io::BufReader::new(obj_bytes);
        let obj::ObjData {
            texture,
            normal,
            position,
            objects,
            ..
        } = obj::ObjData::load_buf(&mut reader)?;

        let vertices: Vec<_> = objects
            .into_iter()
            .flat_map(|object| object.groups)
            .flat_map(|group| group.polys)
            .flat_map(|polygon| polygon.0)
            .map(
                |obj::IndexTuple(position_index, texture_index, normal_index)| {
                    let texture_index = texture_index.unwrap();
                    let normal_index = normal_index.unwrap();

                    // We need to flip uvs because of the way textures work or something..
                    let mut uv: Vec2 = texture[texture_index].into();
                    uv.y = 1.0 - uv.y;

                    Vertex {
                        position: position[position_index].into(),
                        normal: normal[normal_index].into(),
                        uv,
                    }
                },
            )
            .collect();

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });

        Ok(Self {
            buffer,
            num_vertices: vertices.len() as u32,
        })
    }
}

fn load_texture(
    bytes: &[u8],
    bind_group_layout: &wgpu::BindGroupLayout,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
) -> anyhow::Result<wgpu::BindGroup> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.into_rgba();

    let temp_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cheese load_texture buffer"),
        contents: &*image,
        usage: wgpu::BufferUsage::COPY_SRC,
    });

    let texture_extent = wgpu::Extent3d {
        width: image.width(),
        height: image.height(),
        depth: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: texture_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TEXTURE_FORMAT,
        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        label: Some("Cheese texture"),
    });

    encoder.copy_buffer_to_texture(
        wgpu::BufferCopyView {
            buffer: &temp_buf,
            layout: wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * image.width(),
                rows_per_image: 0,
            },
        },
        wgpu::TextureCopyView {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        texture_extent,
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&view),
        }],
    }))
}
