use super::{colour_state_descriptor, DynamicBuffer, RenderContext, DEPTH_FORMAT, INDEX_FORMAT};
use crate::assets::Assets;
use ultraviolet::{Vec2, Vec4};

const WHITE: Vec4 = Vec4::new(1.0, 1.0, 1.0, 1.0);

pub struct LinesPipeline {
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

impl LinesPipeline {
    pub fn new(context: &RenderContext, assets: &Assets) -> Self {
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

        let bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Cheese line bind group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &context.screen_dimension_uniform_buffer,
                            offset: 0,
                            size: None,
                        },
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&context.sampler),
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
            color_states: &[colour_state_descriptor(true), colour_state_descriptor(false)],
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
                    attributes: &wgpu::vertex_attr_array![0 => Float2, 1 => Float2, 2 => Float4, 3 => Int],
                }],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Self {
            bind_group,
            pipeline,
        }
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        line_buffers: &'a LineBuffers,
        assets: &'a Assets,
    ) {
        if let Some((vertices, indices, num_indices)) = line_buffers.get() {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_bind_group(1, &assets.buttons_texture, &[]);
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
    colour: Vec4,
}

impl StrokeVertexConstructor<Vertex> for Constructor {
    fn new_vertex(&mut self, point: Point, _: StrokeAttributes) -> Vertex {
        Vertex {
            position: Vec2::new(point.x, point.y),
            uv: Vec2::new(0.0, 0.0),
            colour: self.colour,
            mode: Mode::SolidColour as i32,
        }
    }
}

impl BasicVertexConstructor<Vertex> for Constructor {
    fn new_vertex(&mut self, point: Point) -> Vertex {
        Vertex {
            position: Vec2::new(point.x, point.y),
            uv: Vec2::new(0.0, 0.0),
            colour: self.colour,
            mode: Mode::SolidColour as i32,
        }
    }
}

pub struct LineBuffers {
    vertices: DynamicBuffer<Vertex>,
    indices: DynamicBuffer<u32>,
    lyon_buffers: VertexBuffers<Vertex, u16>,
}

pub enum Image {
    BuildPump,
    BuildArmoury,
    RecruitEngineer,
    RecruitMouseMarine,
    SetRecruitmentWaypoint,
    CheeseCoins,
}

impl Image {
    fn uv(&self) -> (Vec2, Vec2) {
        match self {
            Self::BuildPump => (Vec2::new(0.0, 0.0), Vec2::new(0.25, 0.5)),
            Self::BuildArmoury => (Vec2::new(0.25, 0.0), Vec2::new(0.25, 0.5)),
            Self::RecruitEngineer => (Vec2::new(0.0, 0.5), Vec2::new(0.25, 0.5)),
            Self::RecruitMouseMarine => (Vec2::new(0.25, 0.5), Vec2::new(0.25, 0.5)),
            Self::SetRecruitmentWaypoint => (Vec2::new(0.5, 0.0), Vec2::new(0.25, 0.5)),
            Self::CheeseCoins => (Vec2::new(0.75, 0.5), Vec2::new(0.125, 0.25)),
        }
    }
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
        colour: Vec4,
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

        self.buffer();
    }

    pub fn draw_image(
        &mut self,
        center: Vec2,
        mut dimensions: Vec2,
        image: Image,
        greyscale: bool,
        dpi_scaling: f32,
    ) {
        dimensions *= dpi_scaling;
        let top_left = center - dimensions / 2.0;

        let (uv_top_left, uv_dimensions) = image.uv();

        let num_vertices = self.vertices.len_waiting();

        let vertices = [
            Vertex::new_textured(top_left, uv_top_left, greyscale),
            Vertex::new_textured(
                top_left + Vec2::new(dimensions.x, 0.0),
                uv_top_left + Vec2::new(uv_dimensions.x, 0.0),
                greyscale,
            ),
            Vertex::new_textured(
                top_left + Vec2::new(0.0, dimensions.y),
                uv_top_left + Vec2::new(0.0, uv_dimensions.y),
                greyscale,
            ),
            Vertex::new_textured(
                top_left + dimensions,
                uv_top_left + uv_dimensions,
                greyscale,
            ),
        ];

        let indices = [0, 1, 2, 1, 2, 3];

        for vertex in &vertices {
            self.vertices.push(*vertex);
        }

        for index in &indices {
            self.indices.push(*index + num_vertices as u32);
        }
    }

    pub fn draw_rect(&mut self, top_left: Vec2, bottom_right: Vec2, dpi_scaling: f32) {
        let dimensions = bottom_right - top_left;

        let mut options = StrokeOptions::default();
        options.line_width = dpi_scaling;

        stroke_rectangle(
            &rect(top_left.x, top_left.y, dimensions.x, dimensions.y),
            &options,
            &mut BuffersBuilder::new(&mut self.lyon_buffers, Constructor { colour: WHITE }),
        )
        .unwrap();

        self.buffer();
    }

    fn buffer(&mut self) {
        let num_vertices = self.vertices.len_waiting();

        for vertex in self.lyon_buffers.vertices.drain(..) {
            self.vertices.push(vertex);
        }

        for index in self.lyon_buffers.indices.drain(..) {
            self.indices.push(index as u32 + num_vertices as u32);
        }
    }

    pub fn upload(&mut self, context: &RenderContext) {
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

enum Mode {
    SolidColour = 0,
    Textured = 1,
    Greyscale = 2,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct Vertex {
    position: Vec2,
    uv: Vec2,
    colour: Vec4,
    mode: i32,
}

impl Vertex {
    fn new_textured(position: Vec2, uv: Vec2, greyscale: bool) -> Self {
        Self {
            position,
            uv,
            colour: Vec4::one(),
            mode: if greyscale {
                Mode::Greyscale
            } else {
                Mode::Textured
            } as i32,
        }
    }
}
