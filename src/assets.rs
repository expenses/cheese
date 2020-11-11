use crate::renderer::{AnimatedVertex, Vertex, TEXTURE_FORMAT};
use ultraviolet::{Vec3, Vec4, Mat4};
use wgpu::util::DeviceExt;

pub struct Assets {
    pub surface_model: Model,
    pub bullet_model: Model,
    pub mouse_model: Model,
    pub mouse_helmet_model: Model,
    pub torus_model: Model,
    pub gltf_model: AnimatedModel,

    pub texture_bind_group_layout: wgpu::BindGroupLayout,

    pub surface_texture: wgpu::BindGroup,
    pub colours_texture: wgpu::BindGroup,
    pub hud_texture: wgpu::BindGroup,
    pub mouse_texture: wgpu::BindGroup,
    pub character_texture: wgpu::BindGroup,
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
            mouse_model: Model::load_gltf(
                include_bytes!("../models/mouse.gltf"),
                "Cheese mouse model",
                device,
            )?,
            mouse_helmet_model: Model::load_gltf(
                include_bytes!("../models/mouse_helmet.gltf"),
                "Cheese mouse helmet model",
                device,
            )?,
            torus_model: Model::load_gltf(
                include_bytes!("../models/torus.gltf"),
                "Cheese torus model",
                device,
            )?,
            gltf_model: AnimatedModel::load_gltf(
                include_bytes!("../animation/character.gltf"),
                "X",
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
            character_texture: load_texture(
                include_bytes!("../animation/Character Texture.png"),
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

                indices.extend(reader.read_indices().unwrap().into_u32());
            }
        }

        log::debug!(
            "Gltf model {} loaded. Vertices: {}. Indices: {}",
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
                label: None,
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
    pub inverse_bind_matrices: Vec<Mat4>,
    pub joints: crate::animation::JointTree,
    pub animations: Vec<crate::animation::Animation>,
}

impl AnimatedModel {
    pub fn load_gltf(
        gltf_bytes: &'static [u8],
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
                let joints = reader.read_joints(0).unwrap().into_u16();
                let weights = reader.read_weights(0).unwrap().into_f32();

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
                            joints: Vec4::new(j[0] as f32, j[1] as f32, j[2] as f32, j[3] as f32),
                            joint_weights: w.into(),
                        });
                    });

                indices.extend(reader.read_indices().unwrap().into_u32());
            }
        }

        debug_assert!(gltf.skins().len() == 1);
        let skin = gltf.skins().next().unwrap();

        let reader = skin.reader(|buffer| Some(&buffers[buffer.index()]));
        let inverse_bind_matrices: Vec<Mat4> = reader
            .read_inverse_bind_matrices()
            .unwrap()
            .map(|matrix| matrix.into())
            .collect();

        let joints = crate::animation::JointTree::from_skin(&skin, &inverse_bind_matrices);

        let mut animations = Vec::new();        

        for animation in gltf.animations() {
            log::debug!("Channels: {}. Samplers: {}", animation.channels().count(), animation.samplers().count());

            assert_eq!(animation.channels().count(), joints.len() * 3);

            let mut channels = animation.channels();

            let mut animation = crate::animation::Animation {
                inputs: Vec::new(),
                outputs: vec![Vec::new(); joints.len()]
            };

            for i in 0 .. joints.len() {
                let reader_a = channels.next().unwrap().reader(|buffer| Some(&buffers[buffer.index()]));
                let reader_b = channels.next().unwrap().reader(|buffer| Some(&buffers[buffer.index()]));
                // Would be scale but we don't care about that.
                let _ = channels.next().unwrap();

                if i == 0 {
                    animation.inputs = reader_a.read_inputs().unwrap().collect();
                }

                let translations = match reader_a.read_outputs().unwrap() {
                    gltf::animation::util::ReadOutputs::Translations(translations) => translations,
                    _ => panic!()
                };

                let rotations = match reader_b.read_outputs().unwrap() {
                    gltf::animation::util::ReadOutputs::Rotations(rotations) => rotations.into_f32(),
                    _ => panic!()
                };

                assert_eq!(translations.clone().count(), rotations.clone().count());

                animation.outputs[i] = translations.zip(rotations)
                    .map(|(t, r)| crate::animation::AnimationTransform {
                        translation: t.into(),
                        rotation: r.into(),
                    }).collect();
            }

            animations.push(animation);
            assert!(channels.next().is_none());
        }


        log::debug!(
            "Animated gltf model {} loaded. Vertices: {}. Indices: {}. Joints: {}. Animations: {}",
            label,
            vertices.len(),
            indices.len(),
            joints.len(),
            animations.len(),
        );

        Ok(Self {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsage::VERTEX,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsage::INDEX,
            }),
            inverse_bind_matrices,
            num_indices: indices.len() as u32,
            joints, animations,
        })
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
