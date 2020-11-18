use super::{colour_state_descriptor, DynamicBuffer, RenderContext, DEPTH_FORMAT};
use std::sync::Arc;
use ultraviolet::{Vec2, Vec3, Vec4};

// A pipeline for drawing 3d lines using `wgpu::PrimitiveTopology::LineList`. This primitive type
// is honestly pretty useless as it doesn't scale with dpi. It's used here for debugging.

pub struct Lines3dPipeline {
    pipeline: wgpu::RenderPipeline,
    main_bind_group: Arc<wgpu::BindGroup>,
}

impl Lines3dPipeline {
    pub fn new(context: &RenderContext) -> Self {
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Cheese lines 3d pipeline layout"),
                    bind_group_layouts: &[&context.main_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let vs = wgpu::include_spirv!("../../shaders/compiled/lines_3d.vert.spv");
        let vs_module = context.device.create_shader_module(vs);

        let pipeline = context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Cheese lines 3d pipeline"),
                layout: Some(&pipeline_layout),
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_module,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &context.fs_transparent_module,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    cull_mode: wgpu::CullMode::Back,
                    ..Default::default()
                }),
                primitive_topology: wgpu::PrimitiveTopology::LineList,
                color_states: &[colour_state_descriptor(true)],
                depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilStateDescriptor::default(),
                }),
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: wgpu::IndexFormat::Uint32,
                    vertex_buffers: &[wgpu::VertexBufferDescriptor {
                        stride: std::mem::size_of::<Lines3dVertex>() as u64,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float4],
                    }],
                },
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });

        Self {
            pipeline,
            main_bind_group: context.main_bind_group.clone(),
        }
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        lines_buffer: &'a DynamicBuffer<Lines3dVertex>,
    ) {
        if let Some((slice, num)) = lines_buffer.get() {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.main_bind_group, &[]);
            render_pass.set_vertex_buffer(0, slice);
            render_pass.draw(0..num, 0..1);
        }
    }
}

pub struct Lines3dBuffer {
    pub lines: DynamicBuffer<Lines3dVertex>,
}

impl Lines3dBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            lines: DynamicBuffer::new(
                device,
                50,
                "Cheese lines 3d buffer",
                wgpu::BufferUsage::VERTEX,
            ),
        }
    }

    pub fn upload(&mut self, context: &RenderContext) {
        self.lines.upload(context);
    }

    pub fn draw_line(&mut self, a: Vec2, b: Vec2, height: f32, colour: Vec4) {
        self.lines.push(Lines3dVertex {
            position: Vec3::new(a.x, height, a.y),
            colour,
        });
        self.lines.push(Lines3dVertex {
            position: Vec3::new(b.x, height, b.y),
            colour,
        });
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct Lines3dVertex {
    position: Vec3,
    colour: Vec4,
}
