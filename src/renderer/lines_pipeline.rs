use super::{colour_state_descriptor, DynamicBuffer, RenderContext, DEPTH_FORMAT, INDEX_FORMAT};
use crate::assets::Assets;
use ultraviolet::{Vec2, Vec3};
use wgpu::util::DeviceExt;

pub struct LinesPipeline {
    uniforms_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    hud_buffer: wgpu::Buffer,
}

impl LinesPipeline {
    pub fn new(context: &RenderContext, assets: &Assets) -> Self {
        let dimensions = context.window.inner_size();
        let width = dimensions.width;
        let height = dimensions.height;
        let sampler = &context.sampler;

        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let uniforms_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Cheese line uniforms buffer"),
                    contents: bytemuck::bytes_of(&Uniforms::new(width, height)),
                    usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
                });

        let bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
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

        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Cheese line pipeline layout"),
                    bind_group_layouts: &[&bind_group_layout, &assets.texture_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let vs = wgpu::include_spirv!("../../shaders/compiled/lines.vert.spv");
        let vs_module = context.device.create_shader_module(vs);

        let fs = wgpu::include_spirv!("../../shaders/compiled/lines.frag.spv");
        let fs_module = context.device.create_shader_module(fs);

        let pipeline = context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
            color_states: &[colour_state_descriptor(true)],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilStateDescriptor::default(),
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: INDEX_FORMAT,
                vertex_buffers: &[wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float2, 1 => Float2, 2 => Float3, 3 => Int],
                }],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let hud_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cheese line hud buffer"),
                contents: bytemuck::cast_slice(&generate_hud_vertices(width, height)),
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            });

        Self {
            uniforms_buffer,
            bind_group,
            pipeline,
            hud_buffer,
        }
    }

    pub fn resize(&self, context: &RenderContext, width: u32, height: u32) {
        context.queue.write_buffer(
            &self.uniforms_buffer,
            0,
            bytemuck::bytes_of(&Uniforms::new(width, height)),
        );
        context.queue.write_buffer(
            &self.hud_buffer,
            0,
            bytemuck::cast_slice(&generate_hud_vertices(width, height)),
        );
    }

    pub fn render_hud<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, assets: &'a Assets) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, &assets.misc_texture, &[]);
        render_pass.set_vertex_buffer(0, self.hud_buffer.slice(..));
        render_pass.draw(0..6, 0..1);
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        line_buffers: &'a LineBuffers,
    ) {
        if let Some((vertices, indices, num_indices)) = line_buffers.get() {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertices);
            render_pass.set_index_buffer(indices);
            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        }
    }
}

use lyon_tessellation::{
    basic_shapes::{fill_rectangle, stroke_rectangle},
    math::{rect, Point},
    BasicVertexConstructor, BuffersBuilder, FillOptions, StrokeAttributes, StrokeOptions,
    StrokeVertexConstructor, VertexBuffers,
};

struct Constructor {
    colour: Vec3,
}

impl StrokeVertexConstructor<Vertex> for Constructor {
    fn new_vertex(&mut self, point: Point, _: StrokeAttributes) -> Vertex {
        Vertex {
            position: Vec2::new(point.x, point.y),
            uv: Vec2::new(0.0, 0.0),
            colour: self.colour,
            textured: false as i32,
        }
    }
}

impl BasicVertexConstructor<Vertex> for Constructor {
    fn new_vertex(&mut self, point: Point) -> Vertex {
        Vertex {
            position: Vec2::new(point.x, point.y),
            uv: Vec2::new(0.0, 0.0),
            colour: self.colour,
            textured: false as i32,
        }
    }
}

pub struct LineBuffers {
    vertices: DynamicBuffer<Vertex>,
    indices: DynamicBuffer<u32>,
    lyon_buffers: VertexBuffers<Vertex, u16>,
}

impl LineBuffers {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            vertices: DynamicBuffer::new(
                device,
                200,
                "Cheese line vertex buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            indices: DynamicBuffer::new(
                device,
                400,
                "Cheese line index buffer",
                wgpu::BufferUsage::INDEX,
            ),
            lyon_buffers: VertexBuffers::new(),
        }
    }

    pub fn draw_filled_rect(
        &mut self,
        center: Vec2,
        mut dimensions: Vec2,
        colour: Vec3,
        dpi_scaling: f32,
    ) {
        dimensions *= dpi_scaling;
        let top_left = center - dimensions / 2.0;

        fill_rectangle(
            &rect(top_left.x, top_left.y, dimensions.x, dimensions.y),
            &FillOptions::default(),
            &mut BuffersBuilder::new(&mut self.lyon_buffers, Constructor { colour }),
        )
        .unwrap();
    }

    pub fn draw_rect(&mut self, top_left: Vec2, bottom_right: Vec2, dpi_scaling: f32) {
        let dimensions = bottom_right - top_left;

        let mut options = StrokeOptions::default();
        options.line_width = dpi_scaling;

        stroke_rectangle(
            &rect(top_left.x, top_left.y, dimensions.x, dimensions.y),
            &options,
            &mut BuffersBuilder::new(
                &mut self.lyon_buffers,
                Constructor {
                    colour: Vec3::new(1.0, 1.0, 1.0),
                },
            ),
        )
        .unwrap();
    }

    pub fn upload(&mut self, context: &RenderContext) {
        for vertex in self.lyon_buffers.vertices.drain(..) {
            self.vertices.push(vertex);
        }

        for index in self.lyon_buffers.indices.drain(..) {
            self.indices.push(index as u32);
        }

        self.vertices.upload(context);
        self.indices.upload(context);
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
    colour: Vec3,
    textured: i32,
}

fn generate_hud_vertices(screen_width: u32, screen_height: u32) -> [Vertex; 6] {
    let vertex = |x, y, u, v| Vertex {
        position: Vec2::new(x as f32, y as f32),
        uv: Vec2::new(u as f32, v),
        colour: Vec3::new(0.0, 0.0, 0.0),
        textured: true as i32,
    };

    let screen_height = screen_height as f32;
    // The hud is a 64px x 8px image
    let hud_height = screen_width as f32 / 8.0;
    let hud_top = screen_height - hud_height;

    let offset = (64.0 - 8.0) / 64.0;

    let top_left = vertex(0, hud_top, 0, offset);
    let top_right = vertex(screen_width, hud_top, 1, offset);
    let bottom_left = vertex(0, screen_height, 0, 1.0);
    let bottom_right = vertex(screen_width, screen_height, 1, 1.0);

    [
        top_left,
        top_right,
        bottom_left,
        top_right,
        bottom_left,
        bottom_right,
    ]
}
