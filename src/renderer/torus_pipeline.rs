use super::{draw_model, DynamicBuffer, RenderContext, Vertex, DEPTH_FORMAT, DISPLAY_FORMAT};
use crate::assets::Model;
use std::sync::Arc;
use ultraviolet::Vec3;

// I want to draw lots of toruses (I'd use tori but then you'd have to look that word up) with
// varying major radii but the same minor radii efficiently. Because of this last part you can't
// just scale the model, so you have to run some different code in the vertex shader.
// See shaders/torus.vert for more.

pub struct TorusPipeline {
    pipeline: wgpu::RenderPipeline,
    main_bind_group: Arc<wgpu::BindGroup>,
}

impl TorusPipeline {
    pub fn new(context: &RenderContext) -> Self {
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Cheese torus pipeline layout"),
                    bind_group_layouts: &[&context.main_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let vs = wgpu::include_spirv!("../../shaders/compiled/torus.vert.spv");
        let vs_module = context.device.create_shader_module(vs);

        let fs = wgpu::include_spirv!("../../shaders/compiled/torus.frag.spv");
        let fs_module = context.device.create_shader_module(fs);

        let pipeline = context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Cheese torus pipeline"),
            layout: Some(&pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                cull_mode: wgpu::CullMode::Back,
                ..Default::default()
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: DISPLAY_FORMAT,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilStateDescriptor::default(),
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint32,
                vertex_buffers: &[
                    wgpu::VertexBufferDescriptor {
                        stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float3, 2 => Float2],
                    },
                    wgpu::VertexBufferDescriptor {
                        stride: std::mem::size_of::<TorusInstance>() as u64,
                        step_mode: wgpu::InputStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![3 => Float3, 4 => Float3, 5 => Float],
                    },
                ],
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
        torus_buffer: &'a DynamicBuffer<TorusInstance>,
        torus_model: &'a Model,
    ) {
        if let Some((slice, num)) = torus_buffer.get() {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.main_bind_group, &[]);
            draw_model(render_pass, torus_model, slice, num);
        }
    }
}

pub struct TorusBuffer {
    pub toruses: DynamicBuffer<TorusInstance>,
}

impl TorusBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            toruses: DynamicBuffer::new(
                device,
                50,
                "Cheese torus buffer",
                wgpu::BufferUsage::VERTEX,
            ),
        }
    }

    pub fn upload(&mut self, context: &RenderContext) {
        self.toruses.upload(context);
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct TorusInstance {
    pub center: Vec3,
    pub colour: Vec3,
    pub radius: f32,
}
