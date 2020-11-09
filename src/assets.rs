use crate::renderer::{Vertex, TEXTURE_FORMAT};
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
            surface_model: Model::load(
                include_bytes!("../models/surface.obj"),
                "Cheese surface model",
                device,
            )?,
            bullet_model: Model::load(
                include_bytes!("../models/bullet.obj"),
                "Cheese bullet model",
                device,
            )?,
            mouse_model: Model::load(
                include_bytes!("../models/mouse.obj"),
                "Cheese mouse model",
                device,
            )?,
            mouse_helmet_model: Model::load(
                include_bytes!("../models/mouse_helmet.obj"),
                "Cheese mouse helmet model",
                device,
            )?,
            torus_model: Model::load(
                include_bytes!("../models/torus.obj"),
                "Cheese torus model",
                device,
            )?,

            surface_texture: load_texture(
                include_bytes!("../textures/surface.png"),
                "Cheese surface texture",
                &texture_bind_group_layout,
                device,
                &mut init_encoder,
            )?,
            colours_texture: load_texture(
                include_bytes!("../textures/colours.png"),
                "Cheese colours texture",
                &texture_bind_group_layout,
                device,
                &mut init_encoder,
            )?,
            hud_texture: load_texture(
                include_bytes!("../textures/hud.png"),
                "Cheese hud texture",
                &texture_bind_group_layout,
                device,
                &mut init_encoder,
            )?,
            mouse_texture: load_texture(
                include_bytes!("../textures/mouse.png"),
                "Cheese mouse texture",
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
    pub indices: Option<Indices>,
}

pub struct Indices {
    pub buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl Model {
    pub fn from_vertices(vertices: Vec<Vertex>, indices: Option<Vec<u32>>, label: &str, device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });

        Self {
            buffer,
            num_vertices: vertices.len() as u32,
            indices: indices.map(|indices| {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsage::INDEX,
                });
                Indices {
                    buffer,
                    num_indices: indices.len() as u32,
                }
            })
        }
    }

    pub fn load(obj_bytes: &[u8], label: &str, device: &wgpu::Device) -> anyhow::Result<Self> {
        let mut reader = std::io::BufReader::new(obj_bytes);
        let obj::ObjData {
            texture,
            normal,
            position,
            objects,
            ..
        } = obj::ObjData::load_buf(&mut reader)?;

        let mut vertices: Vec<_> = objects
            .into_iter()
            .flat_map(|object| object.groups)
            .flat_map(|group| group.polys)
            .flat_map(|polygon| polygon.0)
            .map(
                |obj::IndexTuple(position_index, texture_index, normal_index)| {
                    let texture_index = texture_index.unwrap();
                    let normal_index = normal_index.unwrap();

                    Vertex {
                        position: position[position_index].into(),
                        normal: normal[normal_index].into(),
                        uv: texture[texture_index].into(),
                    }
                },
            )
            .collect();

        // We need to flip uvs because of the way textures work or something..
        // Only on OBJs though, not gltf ;^)
        vertices.iter_mut().for_each(|vertex| vertex.uv.y = 1.0 - vertex.uv.y);

        Ok(Self::from_vertices(vertices, None, label, device))
    }

    pub fn load_gltf(gltf_bytes: &[u8], label: &str, device: &wgpu::Device) -> anyhow::Result<Self> {
        const OCTET_STREAM_URI: &str = "data:application/octet-stream;base64,";
        
        let gltf = gltf::Gltf::from_slice(gltf_bytes)?;

        
        // Load the buffers into a vector of byte vectors.
        // I mostly copied what bevy does for this because it's a little confusing at first.
        // https://github.com/bevyengine/bevy/blob/master/crates/bevy_gltf/src/loader.rs

        let mut buffer_data = Vec::new();
        for buffer in gltf.buffers() {
            match buffer.source() {
                gltf::buffer::Source::Uri(uri) => {
                    if uri.starts_with(OCTET_STREAM_URI) {
                        buffer_data.push(base64::decode(&uri[OCTET_STREAM_URI.len()..])?);
                    } else {
                        return Err(anyhow::anyhow!("Only octet streams are supported with data:"))
                    }
                }
                gltf::buffer::Source::Bin => {
                    if let Some(blob) = gltf.blob.as_deref() {
                        buffer_data.push(blob.into());
                    } else {
                        return Err(anyhow::anyhow!("Missing blob"));
                    }
                }
            }
        }

        let mut vertices = Vec::new();
        let mut indices = None;

        for mesh in gltf.meshes() {
            for primitive in mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles {
                    return Err(anyhow::anyhow!("Primitives with {:?} are not allowed. Triangles only.", primitive.mode()));
                }
                
                let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));
                
                let positions = reader.read_positions().unwrap();
                let tex_coordinates = reader.read_tex_coords(0).unwrap().into_f32();
                let normals = reader.read_normals().unwrap();

                positions.zip(tex_coordinates).zip(normals).for_each(|((p, uv), n)| {
                    vertices.push(Vertex {
                        position: p.into(),
                        normal: n.into(),
                        uv: uv.into()
                    });
                });

                indices = reader.read_indices().map(|indices| indices.into_u32().collect());
            }
        }

        Ok(Self::from_vertices(vertices, indices, label, device))
    }
}

fn load_texture(
    bytes: &[u8],
    label: &str,
    bind_group_layout: &wgpu::BindGroupLayout,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
) -> anyhow::Result<wgpu::BindGroup> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.into_rgba();

    let temp_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cheese texture staging buffer"),
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
        label: Some(label),
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
        label: Some("Cheese texture bind group"),
        layout: bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&view),
        }],
    }))
}
