use super::{
    draw_model, AnimatedVertex, DynamicBuffer, ModelInstance, RenderContext, Vertex, DEPTH_FORMAT,
    INDEX_FORMAT,
};
use crate::assets::{AnimatedModel, Model};
use std::sync::Arc;

pub struct ShadowPipeline {
    static_pipeline: wgpu::RenderPipeline,
    animated_pipeline: wgpu::RenderPipeline,
    bind_group: Arc<wgpu::BindGroup>,
    identity_instance_buffer: Arc<wgpu::Buffer>,
}

impl ShadowPipeline {
    pub fn new(context: &RenderContext) -> Self {
        let vs_shadow_static =
            wgpu::include_spirv!("../../shaders/compiled/shadow_static.vert.spv");
        let vs_shadow_static_module = context.device.create_shader_module(vs_shadow_static);

        let vs_shadow_animated =
            wgpu::include_spirv!("../../shaders/compiled/shadow_animated.vert.spv");
        let vs_shadow_animated_module = context.device.create_shader_module(vs_shadow_animated);

        let static_pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Cheese static shadow pipeline layout"),
                    bind_group_layouts: &[&context.shadow_uniform_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let static_pipeline =
            context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Cheese static shadow pipeline"),
                layout: Some(&static_pipeline_layout),
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_shadow_static_module,
                    entry_point: "main",
                },
                fragment_stage: None,
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    cull_mode: wgpu::CullMode::Back,
                    ..Default::default()
                }),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[],
                depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilStateDescriptor::default(),
                }),
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: INDEX_FORMAT,
                    vertex_buffers: &[
                        wgpu::VertexBufferDescriptor {
                            stride: std::mem::size_of::<Vertex>() as u64,
                            step_mode: wgpu::InputStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float3, 2 => Float2],
                        },
                        wgpu::VertexBufferDescriptor {
                            stride: std::mem::size_of::<ModelInstance>() as u64,
                            step_mode: wgpu::InputStepMode::Instance,
                            attributes: &wgpu::vertex_attr_array![3 => Float4, 4 => Float4, 5 => Float4, 6 => Float4, 7 => Float4],
                        },
                    ],
                },
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });

        let animated_pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Cheese animated shadow pipeline layout"),
                    bind_group_layouts: &[
                        &context.shadow_uniform_bind_group_layout,
                        &context.joint_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let animated_pipeline =
            context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Cheese animated shadow pipeline"),
                layout: Some(&animated_pipeline_layout),
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_shadow_animated_module,
                    entry_point: "main",
                },
                fragment_stage: None,
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    cull_mode: wgpu::CullMode::Back,
                    ..Default::default()
                }),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[],
                depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilStateDescriptor::default(),
                }),
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: INDEX_FORMAT,
                    vertex_buffers: &[
                        wgpu::VertexBufferDescriptor {
                            stride: std::mem::size_of::<AnimatedVertex>() as u64,
                            step_mode: wgpu::InputStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float3, 2 => Float2, 3 => Float4, 4 => Float4],
                        },
                        wgpu::VertexBufferDescriptor {
                            stride: std::mem::size_of::<ModelInstance>() as u64,
                            step_mode: wgpu::InputStepMode::Instance,
                            attributes: &wgpu::vertex_attr_array![5 => Float4, 6 => Float4, 7 => Float4, 8 => Float4, 9 => Float4],
                        },
                    ],
                },
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });

        Self {
            static_pipeline,
            animated_pipeline,
            identity_instance_buffer: context.identity_instance_buffer.clone(),
            bind_group: context.shadow_uniform_bind_group.clone(),
        }
    }

    pub fn render_static<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        model: &'a Model,
        instances: &'a DynamicBuffer<ModelInstance>,
    ) {
        if let Some((slice, num)) = instances.get() {
            render_pass.set_pipeline(&self.static_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            draw_model(render_pass, model, slice, num);
        }
    }

    pub fn render_animated<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        model: &'a AnimatedModel,
        joints: &'a wgpu::BindGroup,
        instances: &'a DynamicBuffer<ModelInstance>,
    ) {
        if let Some((instances, num_instances)) = instances.get() {
            render_pass.set_pipeline(&self.animated_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_bind_group(1, joints, &[]);

            render_pass.set_vertex_buffer(0, model.vertices.slice(..));
            render_pass.set_vertex_buffer(1, instances);
            render_pass.set_index_buffer(model.indices.slice(..));
            render_pass.draw_indexed(0..model.num_indices, 0, 0..num_instances);
        }
    }

    pub fn render_single<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, model: &'a Model) {
        render_pass.set_pipeline(&self.static_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        draw_model(
            render_pass,
            model,
            self.identity_instance_buffer.slice(..),
            1,
        );
    }
}
