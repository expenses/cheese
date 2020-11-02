use ultraviolet::Vec2; 
use super::{GpuBuffer, DISPLAY_FORMAT, DEPTH_FORMAT};
use wgpu::util::DeviceExt;

pub struct Renderer {
	uniforms_buffer: wgpu::Buffer,
	bind_group: wgpu::BindGroup,
	pipeline: wgpu::RenderPipeline,
}

impl Renderer {
	pub fn new(device: &wgpu::Device, width: u32, height: u32) -> (Self, LineBuffers) {
		let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("Cheese line bind group layout"),
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStage::VERTEX,
					ty: wgpu::BindingType::UniformBuffer {
						dynamic: false,
						min_binding_size: None,
					},
					count: None,
				}
			]
		});

		let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Cheese line uniforms buffer"),
			contents: bytemuck::bytes_of(&Uniforms::new(width, height)),
			usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST
		});

		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("Cheese line bind group"),
			layout: &bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::Buffer(uniforms_buffer.slice(..))
				}
			]
		});

		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Cheese line pipeline layout"),
			bind_group_layouts: &[&bind_group_layout],
			push_constant_ranges: &[]
		});

		let vs = wgpu::include_spirv!("../../shaders/lines.vert.spv");
		let vs_module = device.create_shader_module(vs);
	
		let fs = wgpu::include_spirv!("../../shaders/lines.frag.spv");
		let fs_module = device.create_shader_module(fs);

		let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Cheese line pipeline"),
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
			depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
				format: DEPTH_FORMAT,
				depth_write_enabled: false,
				depth_compare: wgpu::CompareFunction::Always,
				stencil: wgpu::StencilStateDescriptor::default(),
			}),
			vertex_state: wgpu::VertexStateDescriptor {
				index_format: wgpu::IndexFormat::Uint16,
				vertex_buffers: &[
					wgpu::VertexBufferDescriptor {
						stride: std::mem::size_of::<Vertex>() as u64,
						step_mode: wgpu::InputStepMode::Vertex,
						attributes: &wgpu::vertex_attr_array![0 => Float2]
					}
				]
			},
			sample_count: 1,
			sample_mask: !0,
			alpha_to_coverage_enabled: false,
		});

		(
			Self {
				uniforms_buffer, bind_group, pipeline
			},
			LineBuffers {
				vertices: GpuBuffer::new(device, 50, "Cheese line vertex buffer", wgpu::BufferUsage::VERTEX),
				indices: GpuBuffer::new(device, 50, "Cheese line index buffer", wgpu::BufferUsage::INDEX),
				lyon_buffers: VertexBuffers::new(),
			}
		)
	}

	pub fn resize(&self, queue: &wgpu::Queue, width: u32, height: u32) {
		queue.write_buffer(
			&self.uniforms_buffer, 0, bytemuck::bytes_of(&Uniforms::new(width, height))
		);
	}

	pub fn render<'a>(
		&'a self, render_pass: &mut wgpu::RenderPass<'a>, line_buffers: &'a mut LineBuffers,
		device: &wgpu::Device, queue: &wgpu::Queue
	) {
		line_buffers.vertices.upload(device, queue);
		line_buffers.indices.upload(device, queue);


		if let Some((vertices, indices, num_indices)) = line_buffers.get() {
			render_pass.set_pipeline(&self.pipeline);
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.set_vertex_buffer(0, vertices);
			render_pass.set_index_buffer(indices);
			render_pass.draw_indexed(0 .. num_indices, 0, 0 .. 1);
		}
	}
}

use lyon_tessellation::{
	StrokeVertexConstructor, StrokeAttributes, BuffersBuilder, StrokeOptions, VertexBuffers,
	math::{rect, Point}, basic_shapes::stroke_rectangle
};

struct Constructor;

impl StrokeVertexConstructor<Vertex> for Constructor {
	fn new_vertex(&mut self, point: Point, _: StrokeAttributes) -> Vertex {
		Vertex {
			position: Vec2::new(point.x, point.y)
		}
	}
}

pub struct LineBuffers {
	vertices: GpuBuffer<Vertex>,
	indices: GpuBuffer<u16>,
	lyon_buffers: VertexBuffers<Vertex, u16>,
}

impl LineBuffers {
	pub fn draw_rect(&mut self, top_left: Vec2, bottom_right: Vec2) {
		let dimensions = bottom_right - top_left;

		stroke_rectangle(
			&rect(top_left.x, top_left.y, dimensions.x, dimensions.y),
			&StrokeOptions::default(),
			&mut BuffersBuilder::new(&mut self.lyon_buffers, Constructor)
		).unwrap();

		for vertex in self.lyon_buffers.vertices.drain(..) {
			self.vertices.push(vertex);
		}

		for index in self.lyon_buffers.indices.drain(..) {
			self.indices.push(index);
		}
	}

	fn get(&self) -> Option<(wgpu::BufferSlice, wgpu::BufferSlice, u32)> {
		match (self.vertices.get(), self.indices.get()) {
			(Some((vertices_slice, _)), Some((indices_slice, num_indices))) => Some((vertices_slice, indices_slice, num_indices)),
			_ => None
		}
	}
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct Uniforms {
	screen_dimensions: Vec2,
}

impl Uniforms {
	fn new(width: u32, height: u32) -> Self {
		Self {
			screen_dimensions: Vec2::new(width as f32, height as f32)
		}
	}
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct Vertex {
	position: Vec2,
}
