use super::{GpuBuffer, DEPTH_FORMAT, DISPLAY_FORMAT};
use ultraviolet::Vec2;
use wgpu::util::DeviceExt;

pub struct Renderer {
    uniforms_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    hud_texture: wgpu::BindGroup,
    hud_buffer: wgpu::Buffer,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device, texture_bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler, width: u32, height: u32, hud_texture: wgpu::BindGroup,
    ) -> (Self, LineBuffers) {
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
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None,
                },
            ],
        });

        let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cheese line uniforms buffer"),
            contents: bytemuck::bytes_of(&Uniforms::new(width, height)),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cheese line bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(uniforms_buffer.slice(..)),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Cheese line pipeline layout"),
            bind_group_layouts: &[&bind_group_layout, texture_bind_group_layout],
            push_constant_ranges: &[],
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
                color_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::DstAlpha,
                    operation: wgpu::BlendOperation::Max,
                },
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
                vertex_buffers: &[wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float2, 1 => Float2, 2 => Int],
                }],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let hud_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cheese line hud buffer"),
            contents: bytemuck::cast_slice(&generate_hud_vertices(width, height)),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });

        (
            Self {
                uniforms_buffer,
                bind_group,
                pipeline,
                hud_texture,
                hud_buffer,
            },
            LineBuffers {
                vertices: GpuBuffer::new(
                    device,
                    50,
                    "Cheese line vertex buffer",
                    wgpu::BufferUsage::VERTEX,
                ),
                indices: GpuBuffer::new(
                    device,
                    50,
                    "Cheese line index buffer",
                    wgpu::BufferUsage::INDEX,
                ),
                lyon_buffers: VertexBuffers::new(),
            },
        )
    }

    pub fn resize(&self, queue: &wgpu::Queue, width: u32, height: u32) {
        queue.write_buffer(
            &self.uniforms_buffer,
            0,
            bytemuck::bytes_of(&Uniforms::new(width, height)),
        );
        queue.write_buffer(
            &self.hud_buffer,
            0,
            bytemuck::cast_slice(&generate_hud_vertices(width, height))
        );
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        line_buffers: &'a mut LineBuffers,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        line_buffers.vertices.upload(device, queue);
        line_buffers.indices.upload(device, queue);

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        if let Some((vertices, indices, num_indices)) = line_buffers.get() {
            render_pass.set_vertex_buffer(0, vertices);
            render_pass.set_index_buffer(indices);
            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        }

        render_pass.set_bind_group(1, &self.hud_texture, &[]);
        render_pass.set_vertex_buffer(0, self.hud_buffer.slice(..));
        render_pass.draw(0 .. 6, 0 .. 1);
    }
}

use lyon_tessellation::{
    basic_shapes::stroke_rectangle,
    math::{rect, Point},
    BuffersBuilder, StrokeAttributes, StrokeOptions, StrokeVertexConstructor, VertexBuffers,
};

struct Constructor;

impl StrokeVertexConstructor<Vertex> for Constructor {
    fn new_vertex(&mut self, point: Point, _: StrokeAttributes) -> Vertex {
        Vertex {
            position: Vec2::new(point.x, point.y),
            uv: Vec2::new(0.0, 0.0),
            textured: false as i32,
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
            &mut BuffersBuilder::new(&mut self.lyon_buffers, Constructor),
        )
        .unwrap();

        for vertex in self.lyon_buffers.vertices.drain(..) {
            self.vertices.push(vertex);
        }

        for index in self.lyon_buffers.indices.drain(..) {
            self.indices.push(index);
        }
    }

    fn get(&self) -> Option<(wgpu::BufferSlice, wgpu::BufferSlice, u32)> {
        match (self.vertices.get(), self.indices.get()) {
            (Some((vertices_slice, _)), Some((indices_slice, num_indices))) => {
                Some((vertices_slice, indices_slice, num_indices))
            }
            _ => None,
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
            screen_dimensions: Vec2::new(width as f32, height as f32),
        }
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct Vertex {
    position: Vec2,
    uv: Vec2,
    textured: i32,
}

fn generate_hud_vertices(screen_width: u32, screen_height: u32) -> [Vertex; 6] {
    let vertex = |x, y, u, v| {
        Vertex {
            position: Vec2::new(x as f32, y as f32),
            uv: Vec2::new(u as f32, v as f32),
            textured: true as i32
        }
    };

    let screen_height = screen_height as f32;
    // The hud is a 64px x 8px image
    let hud_height = screen_width as f32 / 8.0;
    let hud_top = screen_height - hud_height;

    let top_left = vertex(0, hud_top, 0, 0);
    let top_right = vertex(screen_width, hud_top, 1, 0);
    let bottom_left = vertex(0, screen_height, 0, 1);
    let bottom_right = vertex(screen_width, screen_height, 1, 1);

    [
        top_left, top_right, bottom_left,
        top_right, bottom_left, bottom_right
    ]
}
