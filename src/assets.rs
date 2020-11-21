use crate::animation::{Animation, Skin};
use crate::renderer::{AnimatedVertex, Vertex, TEXTURE_FORMAT};
use wgpu::util::DeviceExt;

pub struct Assets {
    pub surface_model: Model,
    pub bullet_model: Model,
    pub mouse_model: AnimatedModel,
    pub mouse_helmet_model: AnimatedModel,
    pub torus_model: Model,
    pub command_indicator_model: Model,
    pub command_path_model: Model,
    pub armoury_model: Model,
    pub cheese_moon_model: Model,
    pub billboard_model: Model,
    pub cheese_droplet_model: Model,
    pub pump_model: AnimatedModel,
    pub pump_static_model: Model,

    pub texture_bind_group_layout: wgpu::BindGroupLayout,

    pub surface_texture: wgpu::BindGroup,
    pub mouse_texture: wgpu::BindGroup,
    pub misc_texture: wgpu::BindGroup,
    pub armoury_texture: wgpu::BindGroup,
    pub pump_texture: wgpu::BindGroup,
}

#[derive(Default)]
pub struct ModelAnimations {
    pub mouse: AnimationInfo,
    pub pump: AnimationInfo,
}

impl Assets {
    pub fn new(
        device: &wgpu::Device,
    ) -> anyhow::Result<(Self, ModelAnimations, wgpu::CommandBuffer)> {
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

        let (mouse_model, mouse_animation_info) = AnimatedModel::load_gltf(
            include_bytes!("../models/mouse.gltf"),
            "Cheese mouse model",
            device,
        )?;

        let (mouse_helmet_model, _) = AnimatedModel::load_gltf(
            include_bytes!("../models/mouse_helmet.gltf"),
            "Cheese mouse helmet model",
            device,
        )?;

        let (pump_model, pump_animation_info) = AnimatedModel::load_gltf(
            include_bytes!("../models/pump.gltf"),
            "Cheese pump model",
            device,
        )?;

        let assets = Self {
            mouse_model,
            mouse_helmet_model,
            pump_model,
            surface_model: Model::load_gltf(
                include_bytes!("../models/surface.gltf"),
                "Cheese surface model",
                device,
            )?,
            bullet_model: Model::load_gltf(
                include_bytes!("../models/bullet.gltf"),
                "Cheese bullet model",
                device,
            )?,
            torus_model: Model::load_gltf(
                include_bytes!("../models/torus.gltf"),
                "Cheese torus model",
                device,
            )?,
            command_indicator_model: Model::load_gltf(
                include_bytes!("../models/command_indicator.gltf"),
                "Cheese command indicator model",
                device,
            )?,
            command_path_model: Model::load_gltf(
                include_bytes!("../models/command_path.gltf"),
                "Cheese command path model",
                device,
            )?,
            armoury_model: Model::load_gltf(
                include_bytes!("../models/armoury.gltf"),
                "Cheese armoury model",
                device,
            )?,
            cheese_moon_model: Model::load_gltf(
                include_bytes!("../models/cheese_moon.gltf"),
                "Cheese cheese moon model",
                device,
            )?,
            billboard_model: Model::load_gltf(
                include_bytes!("../models/billboard.gltf"),
                "Cheese billboard model",
                device,
            )?,
            cheese_droplet_model: Model::load_gltf(
                include_bytes!("../models/cheese_droplet.gltf"),
                "Cheese cheese droplet model",
                device,
            )?,
            pump_static_model: Model::load_gltf(
                include_bytes!("../models/pump.gltf"),
                "Cheese static pump model",
                device,
            )?,

            surface_texture: load_texture(
                include_bytes!("../textures/surface.png"),
                "Cheese surface texture",
                &texture_bind_group_layout,
                device,
                &mut init_encoder,
            )?,
            misc_texture: load_texture(
                include_bytes!("../textures/misc.png"),
                "Cheese misc texture",
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
            armoury_texture: load_texture(
                include_bytes!("../textures/armoury.png"),
                "Cheese armoury texture",
                &texture_bind_group_layout,
                device,
                &mut init_encoder,
            )?,
            pump_texture: load_texture(
                include_bytes!("../textures/pump.png"),
                "Cheese pump texture",
                &texture_bind_group_layout,
                device,
                &mut init_encoder,
            )?,

            texture_bind_group_layout,
        };

        let animations = ModelAnimations {
            mouse: mouse_animation_info,
            pump: pump_animation_info,
        };

        Ok((assets, animations, init_encoder.finish()))
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

pub struct Model {
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub num_indices: u32,
}

impl Model {
    pub fn load_gltf(
        gltf_bytes: &[u8],
        label: &str,
        device: &wgpu::Device,
    ) -> anyhow::Result<Self> {
        let gltf = gltf::Gltf::from_slice(gltf_bytes)?;

        let buffers = load_buffers(&gltf)?;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for mesh in gltf.meshes() {
            for primitive in mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles {
                    return Err(anyhow::anyhow!(
                        "Primitives with {:?} are not allowed. Triangles only.",
                        primitive.mode()
                    ));
                }

                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                let positions = reader.read_positions().unwrap();
                let tex_coordinates = reader.read_tex_coords(0).unwrap().into_f32();
                let normals = reader.read_normals().unwrap();

                let num_vertices = vertices.len() as u32;

                indices.extend(
                    reader
                        .read_indices()
                        .unwrap()
                        .into_u32()
                        .map(|i| i + num_vertices),
                );

                positions
                    .zip(tex_coordinates)
                    .zip(normals)
                    .for_each(|((p, uv), n)| {
                        vertices.push(Vertex {
                            position: p.into(),
                            normal: n.into(),
                            uv: uv.into(),
                        });
                    });
            }
        }

        log::debug!(
            "Gltf model {} loaded. Vertices: {}. Indices: {}.",
            label,
            vertices.len(),
            indices.len(),
        );

        Ok(Self {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsage::VERTEX,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cheese index buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsage::INDEX,
            }),
            num_indices: indices.len() as u32,
        })
    }
}

pub struct AnimatedModel {
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub num_indices: u32,
    pub joint_uniforms: wgpu::Buffer,
}

#[derive(Default)]
pub struct AnimationInfo {
    pub skin: Skin,
    pub animations: Vec<Animation>,
}

#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
#[repr(C)]
struct JointUniforms {
    num_joints: u32,
}

impl AnimatedModel {
    pub fn load_gltf(
        gltf_bytes: &'static [u8],
        label: &str,
        device: &wgpu::Device,
    ) -> anyhow::Result<(Self, AnimationInfo)> {
        let gltf = gltf::Gltf::from_slice(gltf_bytes)?;

        let buffers = load_buffers(&gltf)?;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for mesh in gltf.meshes() {
            for primitive in mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles {
                    return Err(anyhow::anyhow!(
                        "Primitives with {:?} are not allowed. Triangles only.",
                        primitive.mode()
                    ));
                }

                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                let positions = reader.read_positions().unwrap();
                let tex_coordinates = reader.read_tex_coords(0).unwrap().into_f32();
                let normals = reader.read_normals().unwrap();
                let joints = reader.read_joints(0).unwrap().into_u16();
                let weights = reader.read_weights(0).unwrap().into_f32();

                let num_vertices = vertices.len() as u32;

                indices.extend(
                    reader
                        .read_indices()
                        .unwrap()
                        .into_u32()
                        .map(|i| i + num_vertices),
                );

                positions
                    .zip(tex_coordinates)
                    .zip(normals)
                    .zip(joints)
                    .zip(weights)
                    .for_each(|((((p, uv), n), j), w)| {
                        vertices.push(AnimatedVertex {
                            position: p.into(),
                            normal: n.into(),
                            uv: uv.into(),
                            joints: [j[0] as f32, j[1] as f32, j[2] as f32, j[3] as f32].into(),
                            joint_weights: w.into(),
                        });
                    });
            }
        }

        let skin = Skin::load(
            &gltf.skins().next().unwrap(),
            gltf.nodes(),
            &gltf.scenes().next().unwrap(),
            &buffers,
        );

        let animations = crate::animation::load_animations(gltf.animations(), &buffers);

        log::debug!(
            "Gltf model {} loaded. Vertices: {}. Indices: {}. Joints: {}, Animations: {}.",
            label,
            vertices.len(),
            indices.len(),
            skin.joints.len(),
            animations.len(),
        );

        Ok((
            Self {
                vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsage::VERTEX,
                }),
                indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Cheese index buffer"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsage::INDEX,
                }),
                joint_uniforms: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Cheese joint uniform buffer"),
                    contents: bytemuck::bytes_of(&JointUniforms {
                        num_joints: skin.joints.len() as u32,
                    }),
                    usage: wgpu::BufferUsage::UNIFORM,
                }),
                num_indices: indices.len() as u32,
            },
            AnimationInfo { animations, skin },
        ))
    }
}

// Load the buffers from a gltf document into a vector of byte vectors.
// I mostly copied what bevy does for this because it's a little confusing at first.
// https://github.com/bevyengine/bevy/blob/master/crates/bevy_gltf/src/loader.rs
fn load_buffers(gltf: &gltf::Gltf) -> anyhow::Result<Vec<Vec<u8>>> {
    const OCTET_STREAM_URI: &str = "data:application/octet-stream;base64,";

    let mut buffers = Vec::new();

    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Uri(uri) => {
                if uri.starts_with(OCTET_STREAM_URI) {
                    buffers.push(base64::decode(&uri[OCTET_STREAM_URI.len()..])?);
                } else {
                    return Err(anyhow::anyhow!(
                        "Only octet streams are supported with data:"
                    ));
                }
            }
            gltf::buffer::Source::Bin => {
                if let Some(blob) = gltf.blob.as_deref() {
                    buffers.push(blob.into());
                } else {
                    return Err(anyhow::anyhow!("Missing blob"));
                }
            }
        }
    }

    Ok(buffers)
}
