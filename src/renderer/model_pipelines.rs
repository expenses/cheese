use super::{DynamicBuffer, RenderContext, Vertex, DEPTH_FORMAT, DISPLAY_FORMAT};
use crate::assets::{Assets, Model};
use std::sync::Arc;
use ultraviolet::Mat4;
use wgpu::util::DeviceExt;

pub struct ModelPipelines {
    identity_instance_buffer: wgpu::Buffer,
    model_pipeline: wgpu::RenderPipeline,
    transparent_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    main_bind_group: Arc<wgpu::BindGroup>,
}

impl ModelPipelines {
    pub fn new(context: &RenderContext, assets: &Assets) -> Self {
        // Create the shaders and pipeline

        let vs = wgpu::include_spirv!("../../shaders/shader.vert.spv");
        let vs_module = context.device.create_shader_module(vs);

        let fs = wgpu::include_spirv!("../../shaders/shader.frag.spv");
        let fs_module = context.device.create_shader_module(fs);

        let fs_transparent = wgpu::include_spirv!("../../shaders/transparent.frag.spv");
        let fs_transparent_module = context.device.create_shader_module(fs_transparent);

        let model_pipeline = create_render_pipeline(
            &context.device,
            &[
                &context.main_bind_group_layout,
                &assets.texture_bind_group_layout,
            ],
            "Cheese model pipeline",
            wgpu::PrimitiveTopology::TriangleList,
            &vs_module,
            &fs_module,
            false,
        );

        let transparent_pipeline = create_render_pipeline(
            &context.device,
            &[&context.main_bind_group_layout],
            "Cheese transparent model pipeline",
            wgpu::PrimitiveTopology::TriangleList,
            &vs_module,
            &fs_transparent_module,
            true,
        );

        let line_pipeline = create_render_pipeline(
            &context.device,
            &[
                &context.main_bind_group_layout,
                &assets.texture_bind_group_layout,
            ],
            "Cheese line pipeline",
            wgpu::PrimitiveTopology::LineList,
            &vs_module,
            &fs_module,
            false,
        );

        let identity_instance_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::bytes_of(&ModelInstance {
                        transform: Mat4::identity(),
                        uv_x_offset: 0.0,
                    }),
                    usage: wgpu::BufferUsage::VERTEX,
                });

        Self {
            identity_instance_buffer,
            model_pipeline,
            transparent_pipeline,
            line_pipeline,
            main_bind_group: context.main_bind_group.clone(),
        }
    }

    pub fn render_single<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        texture: &'a wgpu::BindGroup,
        model: &'a Model,
    ) {
        render_pass.set_pipeline(&self.model_pipeline);
        render_pass.set_bind_group(0, &self.main_bind_group, &[]);
        render_pass.set_bind_group(1, texture, &[]);
        render_pass.set_vertex_buffer(0, model.buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.identity_instance_buffer.slice(..));
        render_pass.draw(0..model.num_vertices, 0..1);
    }

    pub fn render_instanced<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        instances: &'a DynamicBuffer<ModelInstance>,
        texture: &'a wgpu::BindGroup,
        model: &'a Model,
    ) {
        if let Some((slice, num)) = instances.get() {
            render_pass.set_pipeline(&self.model_pipeline);
            render_pass.set_bind_group(0, &self.main_bind_group, &[]);
            render_pass.set_bind_group(1, texture, &[]);
            render_pass.set_vertex_buffer(0, model.buffer.slice(..));
            render_pass.set_vertex_buffer(1, slice);
            render_pass.draw(0..model.num_vertices, 0..num);
        }
    }

    pub fn render_transparent<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        instances: &'a DynamicBuffer<ModelInstance>,
        model: &'a Model,
    ) {
        if let Some((slice, num)) = instances.get() {
            render_pass.set_pipeline(&self.transparent_pipeline);
            render_pass.set_bind_group(0, &self.main_bind_group, &[]);
            render_pass.set_vertex_buffer(0, model.buffer.slice(..));
            render_pass.set_vertex_buffer(1, slice);
            render_pass.draw(0..model.num_vertices, 0..num);
        }
    }

    pub fn render_lines<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        lines: &'a DynamicBuffer<Vertex>,
        texture: &'a wgpu::BindGroup,
    ) {
        if let Some((slice, num)) = lines.get() {
            render_pass.set_pipeline(&self.line_pipeline);
            render_pass.set_bind_group(0, &self.main_bind_group, &[]);
            render_pass.set_bind_group(1, texture, &[]);
            render_pass.set_vertex_buffer(0, slice);
            render_pass.set_vertex_buffer(1, self.identity_instance_buffer.slice(..));
            render_pass.draw(0..num, 0..1);
        }
    }

    /*pub fn render(
        &self, view: Mat4, instance_buffers: &mut InstanceBuffers, assets: &Assets, context: &RenderContext, render_pass: &mut wgpu::RenderPass,
    ) {
        instance_buffers.mice.upload(&context.device, &context.queue);
        instance_buffers
            .command_paths
            .upload(&context.device, &context.queue);
        instance_buffers.bullets.upload(&context.device, &context.queue);


        render_pass.set_pipeline(&self.model_pipeline);
        render_pass.set_bind_group(0, &self.main_bind_group, &[]);

        // Draw bullets
        render_instanced(
            &mut render_pass, &instance_buffers.bullets,
            &assets.colours_texture, &assets.bullet_model,
        );

        // Draw mice
        render_instanced(
            &mut render_pass, &instance_buffers.mice,
            &assets.mouse_texture, &assets.mouse_model,
        );

        // Draw surface
        render_pass.set_bind_group(1, &assets.surface_texture, &[]);
        render_pass.set_vertex_buffer(0, assets.surface_model.buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.identity_instance_buffer.slice(..));
        render_pass.draw(0..assets.surface_model.num_vertices, 0..1);

        // Draw Command paths
        if let Some((slice, num)) = instance_buffers.command_paths.get() {
            render_pass.set_pipeline(&self.line_pipeline);
            render_pass.set_bind_group(1, &assets.colours_texture, &[]);
            render_pass.set_vertex_buffer(0, slice);
            render_pass.set_vertex_buffer(1, self.identity_instance_buffer.slice(..));
            render_pass.draw(0..num, 0..1);
        }

        // Draw tori
        /*self.torus_renderer.render(
            &mut render_pass,
            &mut instance_buffers.toruses,
            &self.main_bind_group,
            &self.device,
            &self.queue,
        );*/

        // Draw helmets
        render_pass.set_pipeline(&self.transparent_pipeline);
        render_instanced(
            &mut render_pass, &instance_buffers.mice,
            &assets.mouse_texture, &assets.mouse_helmet_model
        );

        /*// Draw UI

        self.line_renderer.render(
            &mut render_pass,
            &mut instance_buffers.line_buffers,
            &self.device,
            &self.queue,
            assets,
        );*/

            /*
            let size = self.window.inner_size();
            let mut staging_belt = wgpu::util::StagingBelt::new(10);

            instance_buffers
                .glyph_brush
                .draw_queued(
                    &self.device,
                    &mut staging_belt,
                    &mut encoder,
                    &frame.output.view,
                    size.width,
                    size.height,
                )
                .unwrap();

            staging_belt.finish();

            self.queue.submit(Some(encoder.finish()));

            // Do I need to do this?
            // staging_belt.recall();
            */
    }*/
}

fn create_render_pipeline(
    device: &wgpu::Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    label: &str,
    primitives: wgpu::PrimitiveTopology,
    vs_module: &wgpu::ShaderModule,
    fs_module: &wgpu::ShaderModule,
    alpha_blend: bool,
) -> wgpu::RenderPipeline {
    let colour_state_descriptor = if alpha_blend {
        wgpu::ColorStateDescriptor {
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
        }
    } else {
        wgpu::ColorStateDescriptor {
            format: DISPLAY_FORMAT,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }
    };

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Cheese pipeline layout"),
        bind_group_layouts,
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some(label),
		layout: Some(&pipeline_layout),
		vertex_stage: wgpu::ProgrammableStageDescriptor {
			module: vs_module,
			entry_point: "main",
		},
		fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
			module: fs_module,
			entry_point: "main",
		}),
		rasterization_state: Some(wgpu::RasterizationStateDescriptor {
			cull_mode: wgpu::CullMode::Back,
			..Default::default()
		}),
		primitive_topology: primitives,
		color_states: &[colour_state_descriptor],
		depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
			format: DEPTH_FORMAT,
			depth_write_enabled: true,
			depth_compare: wgpu::CompareFunction::Less,
			stencil: wgpu::StencilStateDescriptor::default(),
		}),
		vertex_state: wgpu::VertexStateDescriptor {
			index_format: wgpu::IndexFormat::Uint16,
			vertex_buffers: &[
				wgpu::VertexBufferDescriptor {
					stride: std::mem::size_of::<Vertex>() as u64,
					step_mode: wgpu::InputStepMode::Vertex,
					attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float3, 2 => Float2],
				},
				wgpu::VertexBufferDescriptor {
					stride: std::mem::size_of::<ModelInstance>() as u64,
					step_mode: wgpu::InputStepMode::Instance,
					attributes: &wgpu::vertex_attr_array![3 => Float, 4 => Float4, 5 => Float4, 6 => Float4, 7 => Float4],
				},
			],
		},
		sample_count: 1,
		sample_mask: !0,
		alpha_to_coverage_enabled: false,
	})
}

pub struct ModelBuffers {
    pub mice: DynamicBuffer<ModelInstance>,
    pub command_paths: DynamicBuffer<Vertex>,
    pub bullets: DynamicBuffer<ModelInstance>,
}

impl ModelBuffers {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            mice: DynamicBuffer::new(
                device,
                50,
                "Cheese mice instance buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            bullets: DynamicBuffer::new(
                device,
                200,
                "Cheese bullet buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            command_paths: DynamicBuffer::new(
                device,
                50,
                "Cheese command paths buffer",
                wgpu::BufferUsage::VERTEX,
            ),
        }
    }

    pub fn upload(&mut self, context: &RenderContext) {
        self.mice.upload(context);
        self.command_paths.upload(context);
        self.bullets.upload(context);
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct ModelInstance {
    pub uv_x_offset: f32,
    pub transform: Mat4,
}
