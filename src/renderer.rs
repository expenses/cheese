use winit::{
	event_loop::EventLoop,
	window::Window
};
use wgpu::util::DeviceExt;
use ultraviolet::{Vec2, Vec3, Mat4};

const DISPLAY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

pub struct Renderer {
	swap_chain: wgpu::SwapChain,
	window: Window,
	device: wgpu::Device,
	pipeline: wgpu::RenderPipeline,
	queue: wgpu::Queue,
	surface: wgpu::Surface,
	swap_chain_desc: wgpu::SwapChainDescriptor,
	
	uniform_buffer: wgpu::Buffer,
	sampler_and_uniform_bind_group: wgpu::BindGroup,

	surface_model: Model,
}

impl Renderer {
	pub async fn new(event_loop: &EventLoop<()>) -> anyhow::Result<(Self, CpuBuffers)> {
		let window = Window::new(event_loop)?;

		let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
		let surface = unsafe {
			instance.create_surface(&window)
		};

		let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
			power_preference: wgpu::PowerPreference::default(),
			compatible_surface: Some(&surface),
		}).await.ok_or(anyhow::anyhow!("request_adapter failed"))?;

		let (device, queue) = adapter.request_device(
			&wgpu::DeviceDescriptor {
				features: wgpu::Features::empty(),
				limits: wgpu::Limits::default(),
				shader_validation: true
			},
			None,
		).await?;

		// Create bind groups

		let sampler_and_uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("Cheese sampler and uniform bind group layout"),
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStage::VERTEX,
					ty: wgpu::BindingType::UniformBuffer {
						dynamic: false,
						min_binding_size: None,
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStage::FRAGMENT,
					ty: wgpu::BindingType::Sampler {
						comparison: false,
					},
					count: None,
				},
			]
		});

		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			mag_filter: wgpu::FilterMode::Nearest,
			min_filter: wgpu::FilterMode::Nearest,
			label: Some("Cheese Sampler"),
			..Default::default()
		});

		let window_size = window.inner_size();
		
		let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Cheese uniform buffer"),
			contents: bytemuck::bytes_of(&Uniforms::new(window_size.width, window_size.height, 90.0)),
			usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
		});

		let sampler_and_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &sampler_and_uniform_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..))
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(&sampler),
				},
				
			],
			label: Some("Cheese BindGroup"),
		});
		
		// Create bind group for textures

		let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("Cheese texture bind group layout"),
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStage::FRAGMENT,
					ty: wgpu::BindingType::SampledTexture {
						multisampled: false,
						dimension: wgpu::TextureViewDimension::D2,
						component_type: wgpu::TextureComponentType::Float,
					},
					count: None,
				}
			]
		});

		// Load models

		let mut init_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("Cheese init_encoder")
		});

		let surface_model = Model::load(
			include_bytes!("../models/surface.obj"), include_bytes!("../textures/surface.png"),
			&texture_bind_group_layout, &device, &mut init_encoder
		)?;

		queue.submit(Some(init_encoder.finish()));

		// Create the shaders and pipeline

		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Cheese pipeline layout"),
			bind_group_layouts: &[&sampler_and_uniform_bind_group_layout, &texture_bind_group_layout],
			push_constant_ranges: &[]
		});

		let vs = wgpu::include_spirv!("../shaders/shader.vert.spv");
		let vs_module = device.create_shader_module(vs);
	
		let fs = wgpu::include_spirv!("../shaders/shader.frag.spv");
		let fs_module = device.create_shader_module(fs);

		let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Cheese render pipeline"),
			layout: Some(&pipeline_layout),
			vertex_stage: wgpu::ProgrammableStageDescriptor {
				module: &vs_module,
				entry_point: "main",
			},
			fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
			}),
			rasterization_state: Some(wgpu::RasterizationStateDescriptor::default()),
			primitive_topology: wgpu::PrimitiveTopology::TriangleList,
			color_states: &[wgpu::ColorStateDescriptor {
				format: DISPLAY_FORMAT,
				color_blend: wgpu::BlendDescriptor::REPLACE,
				alpha_blend: wgpu::BlendDescriptor::REPLACE,
				write_mask: wgpu::ColorWrite::ALL,
			}],
			depth_stencil_state: None,
			vertex_state: wgpu::VertexStateDescriptor {
				index_format: wgpu::IndexFormat::Uint16,
				vertex_buffers: &[
                    wgpu::VertexBufferDescriptor {
                        stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float3, 2 => Float3],
                    }
                ],
			},
			sample_count: 1,
			sample_mask: !0,
			alpha_to_coverage_enabled: false,
		});

		// Create the swap chain.

		let swap_chain_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: DISPLAY_FORMAT,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
		};
		
		let swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);


		Ok((
			Self {
				swap_chain, window, device, pipeline, queue, sampler_and_uniform_bind_group,
				uniform_buffer, surface, surface_model, swap_chain_desc,
			},
			CpuBuffers
		))
	}

	pub fn resize(&mut self, width: u32, height: u32) {
		self.swap_chain_desc.width = width;
		self.swap_chain_desc.height = height;
		self.swap_chain = self.device.create_swap_chain(&self.surface, &self.swap_chain_desc);

		let uniforms = Uniforms::new(width, height, 90.0);

		self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
	}

	pub fn render(&mut self) {
		if let Ok(frame) = self.swap_chain.get_current_frame() {
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
				label: Some("Cheese render encoder")
			});

			{
				let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.output.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.125, b: 0.125, a: 1.0 }),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
				});
				
				render_pass.set_pipeline(&self.pipeline);
				render_pass.set_bind_group(0, &self.sampler_and_uniform_bind_group, &[]);
				render_pass.set_bind_group(1, &self.surface_model.bind_group, &[]);
				render_pass.set_vertex_buffer(0, self.surface_model.buffer.slice(..));
				render_pass.draw(0 .. self.surface_model.num_vertices, 0 .. 1);
			}

			self.queue.submit(Some(encoder.finish()));
		}
	}

	pub fn request_redraw(&self) {
		self.window.request_redraw();
	}
}

pub struct CpuBuffers;

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct Uniforms {
	perspective: Mat4,
	view: Mat4,
}

impl Uniforms {
	fn new(window_width: u32, window_height: u32, vertical_fov: f32) -> Self {
		Self {
			perspective: ultraviolet::projection::perspective_wgpu_dx(
				vertical_fov,
				window_width as f32 / window_height as f32,
				0.0, 
				1000.0,
			),
			view: Mat4::look_at(
				Vec3::new(1.0, 1.0, 1.0),
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(0.0, 1.0, 0.0),
			)
		}
	}
}

pub struct Model {
	buffer: wgpu::Buffer,
	bind_group: wgpu::BindGroup,
	num_vertices: u32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct Vertex {
	position: Vec3,
	normal: Vec3,
	uv: Vec2,
}

impl Model {
	fn load(
		obj_bytes: &[u8], image_bytes: &[u8], bind_group_layout: &wgpu::BindGroupLayout,
		device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder
	) -> anyhow::Result<Self> {
		let mut reader = std::io::BufReader::new(obj_bytes);
		let obj::ObjData { texture, normal, position, objects, .. } = obj::ObjData::load_buf(&mut reader)?;

		let vertices: Vec<_> = objects.into_iter()
			.flat_map(|object| object.groups)
			.flat_map(|group| group.polys)
			.flat_map(|polygon| {
				polygon.0
			})
			.map(|obj::IndexTuple(position_index, texture_index, normal_index)| {
				let texture_index = texture_index.unwrap();
				let normal_index = normal_index.unwrap();

				Vertex {
					position: position[position_index].into(),
					normal: normal[normal_index].into(),
					uv: texture[texture_index].into(),
				}
			})
			.collect();

		println!("{}", vertices.len());

		let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			contents: bytemuck::cast_slice(&vertices),
			usage: wgpu::BufferUsage::VERTEX
		});

		let texture = load_texture(image_bytes, device, encoder);

		let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&texture),
				},
			],
		});

		Ok(Self {
			buffer, bind_group: texture_bind_group, num_vertices: vertices.len() as u32,
		})
	}
}

fn load_texture(
	bytes: &[u8], device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder,
) -> wgpu::TextureView {
	let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).unwrap()
		.into_rgba();

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
		format: wgpu::TextureFormat::Rgba8Unorm,
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
			}
		},
		wgpu::TextureCopyView {
			texture: &texture,
			mip_level: 0,
			origin: wgpu::Origin3d::ZERO,
		},
		texture_extent,
	);

	texture.create_view(&wgpu::TextureViewDescriptor::default())
}
