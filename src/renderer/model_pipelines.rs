use super::{
    colour_state_descriptor, draw_model, AnimatedVertex, DynamicBuffer, RenderContext,
    StaticBuffer, Vertex, DEPTH_FORMAT,
};
use crate::assets::{AnimatedModel, Assets, Model};
use std::sync::Arc;
use ultraviolet::{Mat4, Vec4};
use wgpu::util::DeviceExt;

pub struct ModelPipelines {
    identity_instance_buffer: Arc<wgpu::Buffer>,
    model_pipeline: wgpu::RenderPipeline,
    animated_pipeline: wgpu::RenderPipeline,
    transparent_animated_pipeline: wgpu::RenderPipeline,
    transparent_textured_bloom_pipeline: wgpu::RenderPipeline,
    transparent_textured_no_depth_pipeline: wgpu::RenderPipeline,
    transparent_pipeline: wgpu::RenderPipeline,
    main_bind_group: Arc<wgpu::BindGroup>,
    shadow_uniform_bind_group: Arc<wgpu::BindGroup>,
}

impl ModelPipelines {
    pub fn new(context: &RenderContext, assets: &Assets) -> Self {
        let vs = wgpu::include_spirv!("../../shaders/compiled/static_model.vert.spv");
        let vs_module = context.device.create_shader_module(vs);

        let vs_animated = wgpu::include_spirv!("../../shaders/compiled/animated_model.vert.spv");
        let vs_animated_module = context.device.create_shader_module(vs_animated);

        let fs = wgpu::include_spirv!("../../shaders/compiled/textured.frag.spv");
        let fs_module = context.device.create_shader_module(fs);

        let fs_transparent_textured =
            wgpu::include_spirv!("../../shaders/compiled/transparent_textured.frag.spv");
        let fs_transparent_textured_module =
            context.device.create_shader_module(fs_transparent_textured);

        let fs_transparent_textured_bloom =
            wgpu::include_spirv!("../../shaders/compiled/transparent_textured_bloom.frag.spv");
        let fs_transparent_textured_bloom_module = context
            .device
            .create_shader_module(fs_transparent_textured_bloom);

        let model_pipeline = create_render_pipeline(
            &context.device,
            &[
                &context.main_bind_group_layout,
                &assets.texture_bind_group_layout,
                &context.shadow_uniform_bind_group_layout,
            ],
            "Cheese model pipeline",
            &vs_module,
            &fs_module,
            false,
            true,
        );

        let animated_pipeline = create_animated_pipeline(
            &context.device,
            &[
                &context.main_bind_group_layout,
                &assets.texture_bind_group_layout,
                &context.joint_bind_group_layout,
                &context.shadow_uniform_bind_group_layout,
            ],
            &vs_animated_module,
            &fs_module,
            false,
        );

        let transparent_animated_pipeline = create_animated_pipeline(
            &context.device,
            &[
                &context.main_bind_group_layout,
                &assets.texture_bind_group_layout,
                &context.joint_bind_group_layout,
                &context.shadow_uniform_bind_group_layout,
            ],
            &vs_animated_module,
            &context.fs_transparent_module,
            true,
        );

        let transparent_textured_bloom_pipeline = create_render_pipeline(
            &context.device,
            &[
                &context.main_bind_group_layout,
                &assets.texture_bind_group_layout,
            ],
            "Cheese transparent textured pipeline",
            &context.vs_transparent_module,
            &fs_transparent_textured_bloom_module,
            true,
            true,
        );

        let transparent_textured_no_depth_pipeline = create_render_pipeline(
            &context.device,
            &[
                &context.main_bind_group_layout,
                &assets.texture_bind_group_layout,
            ],
            "Cheese transparent textured pipeline",
            &context.vs_transparent_module,
            &fs_transparent_textured_module,
            true,
            false,
        );

        let transparent_pipeline = create_render_pipeline(
            &context.device,
            &[&context.main_bind_group_layout],
            "Cheese transparent pipeline",
            &context.vs_transparent_module,
            &context.fs_transparent_module,
            true,
            true,
        );

        Self {
            model_pipeline,
            animated_pipeline,
            transparent_animated_pipeline,
            transparent_textured_bloom_pipeline,
            transparent_textured_no_depth_pipeline,
            transparent_pipeline,
            main_bind_group: context.main_bind_group.clone(),
            identity_instance_buffer: context.identity_instance_buffer.clone(),
            shadow_uniform_bind_group: context.shadow_uniform_bind_group.clone(),
        }
    }

    pub fn render_animated<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        instances: &'a DynamicBuffer<ModelInstance>,
        texture: &'a wgpu::BindGroup,
        model: &'a AnimatedModel,
        joints: &'a wgpu::BindGroup,
    ) {
        if let Some((slice, num)) = instances.get() {
            render_pass.set_pipeline(&self.animated_pipeline);
            render_pass.set_bind_group(0, &self.main_bind_group, &[]);
            render_pass.set_bind_group(1, texture, &[]);
            render_pass.set_bind_group(2, joints, &[]);
            render_pass.set_bind_group(3, &self.shadow_uniform_bind_group, &[]);

            render_pass.set_vertex_buffer(0, model.vertices.slice(..));
            render_pass.set_vertex_buffer(1, slice);
            render_pass.set_index_buffer(model.indices.slice(..));
            render_pass.draw_indexed(0..model.num_indices, 0, 0..num);
        }
    }

    pub fn render_transparent_animated<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        instances: &'a DynamicBuffer<ModelInstance>,
        dummy_texture: &'a wgpu::BindGroup,
        model: &'a AnimatedModel,
        joints: &'a wgpu::BindGroup,
    ) {
        if let Some((slice, num)) = instances.get() {
            render_pass.set_pipeline(&self.transparent_animated_pipeline);
            render_pass.set_bind_group(0, &self.main_bind_group, &[]);
            // Needed for bind group reasons
            // (basically because I don't want to have 2 animation vertex shaders)
            render_pass.set_bind_group(1, dummy_texture, &[]);
            render_pass.set_bind_group(2, joints, &[]);
            render_pass.set_bind_group(3, &self.shadow_uniform_bind_group, &[]);

            render_pass.set_vertex_buffer(0, model.vertices.slice(..));
            render_pass.set_vertex_buffer(1, slice);
            render_pass.set_index_buffer(model.indices.slice(..));
            render_pass.draw_indexed(0..model.num_indices, 0, 0..num);
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
        render_pass.set_bind_group(2, &self.shadow_uniform_bind_group, &[]);
        draw_model(
            render_pass,
            model,
            self.identity_instance_buffer.slice(..),
            1,
        );
    }

    pub fn render_single_with_transform<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        model: &'a Model,
        texture: &'a wgpu::BindGroup,
        transform: &'a StaticBuffer<ModelInstance>,
    ) {
        render_pass.set_pipeline(&self.model_pipeline);
        render_pass.set_bind_group(0, &self.main_bind_group, &[]);
        render_pass.set_bind_group(1, texture, &[]);
        render_pass.set_bind_group(2, &self.shadow_uniform_bind_group, &[]);
        draw_model(render_pass, model, transform.buffer.slice(..), 1);
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
            render_pass.set_bind_group(2, &self.shadow_uniform_bind_group, &[]);
            draw_model(render_pass, model, slice, num);
        }
    }

    pub fn render_transparent_textured_with_bloom<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        instances: &'a DynamicBuffer<ModelInstance>,
        texture: &'a wgpu::BindGroup,
        model: &'a Model,
    ) {
        if let Some((slice, num)) = instances.get() {
            render_pass.set_pipeline(&self.transparent_textured_bloom_pipeline);
            render_pass.set_bind_group(0, &self.main_bind_group, &[]);
            render_pass.set_bind_group(1, texture, &[]);
            draw_model(render_pass, model, slice, num);
        }
    }

    pub fn render_transparent_textured_without_depth<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        instances: &'a DynamicBuffer<ModelInstance>,
        texture: &'a wgpu::BindGroup,
        model: &'a Model,
    ) {
        if let Some((slice, num)) = instances.get() {
            render_pass.set_pipeline(&self.transparent_textured_no_depth_pipeline);
            render_pass.set_bind_group(0, &self.main_bind_group, &[]);
            render_pass.set_bind_group(1, texture, &[]);
            draw_model(render_pass, model, slice, num);
        }
    }

    pub fn render_transparent_buffer<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        model: &'a Model,
        instances: &'a wgpu::Buffer,
        num_instances: u32,
    ) {
        if num_instances > 0 {
            render_pass.set_pipeline(&self.transparent_pipeline);
            render_pass.set_bind_group(0, &self.main_bind_group, &[]);
            draw_model(render_pass, model, instances.slice(..), num_instances)
        }
    }
}

fn create_render_pipeline(
    device: &wgpu::Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    label: &str,
    vs_module: &wgpu::ShaderModule,
    fs_module: &wgpu::ShaderModule,
    alpha_blend: bool,
    write_depth: bool,
) -> wgpu::RenderPipeline {
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
		primitive_topology: wgpu::PrimitiveTopology::TriangleList,
		color_states: &[colour_state_descriptor(alpha_blend), colour_state_descriptor(alpha_blend)],
		depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
			format: DEPTH_FORMAT,
			depth_write_enabled: write_depth,
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
					stride: std::mem::size_of::<ModelInstance>() as u64,
					step_mode: wgpu::InputStepMode::Instance,
					attributes: &wgpu::vertex_attr_array![3 => Float4, 4 => Float4, 5 => Float4, 6 => Float4, 7 => Float4],
				},
			],
		},
		sample_count: 1,
		sample_mask: !0,
		alpha_to_coverage_enabled: false,
	})
}

fn create_animated_pipeline(
    device: &wgpu::Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    vs_module: &wgpu::ShaderModule,
    fs_module: &wgpu::ShaderModule,
    alpha_blend: bool,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Cheese animated pipeline layout"),
        bind_group_layouts,
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("Cheese animated pipeline"),
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
		primitive_topology: wgpu::PrimitiveTopology::TriangleList,
		color_states: &[colour_state_descriptor(alpha_blend), colour_state_descriptor(false)],
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
	})
}

pub struct BuildingPlan {
    building: Option<crate::ecs::Building>,
    buffer: StaticBuffer<ModelInstance>,
}

impl BuildingPlan {
    pub fn clear(&mut self) {
        self.building = None;
    }

    pub fn set(&mut self, building: crate::ecs::Building, instance: ModelInstance) {
        self.building = Some(building);
        self.buffer.write(instance);
    }

    fn upload(&self, context: &RenderContext) {
        self.buffer.upload(context);
    }

    pub fn get(&self) -> Option<(crate::ecs::Building, &wgpu::Buffer)> {
        self.building
            .map(|building| (building, &self.buffer.buffer))
    }
}

fn create_joint_bind_group(
    context: &RenderContext,
    label: &str,
    joint_buffer: &DynamicBuffer<Mat4>,
    model: &AnimatedModel,
) -> wgpu::BindGroup {
    context
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &context.joint_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &joint_buffer.buffer,
                        offset: 0,
                        size: None,
                    },
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &model.joint_uniforms,
                        offset: 0,
                        size: None,
                    },
                },
            ],
        })
}

pub struct JointBuffer {
    buffer: DynamicBuffer<Mat4>,
    pub bind_group: wgpu::BindGroup,
    bind_group_label: &'static str,
}

impl JointBuffer {
    fn new(
        context: &RenderContext,
        capacity: usize,
        label: &'static str,
        bind_group_label: &'static str,
        model: &AnimatedModel,
    ) -> Self {
        let buffer =
            DynamicBuffer::new(&context.device, capacity, label, wgpu::BufferUsage::STORAGE);

        Self {
            bind_group: create_joint_bind_group(context, bind_group_label, &buffer, model),
            buffer,
            bind_group_label,
        }
    }

    fn upload(&mut self, context: &RenderContext, model: &AnimatedModel) {
        let resized = self.buffer.upload(context);

        if resized {
            self.bind_group =
                create_joint_bind_group(context, &self.bind_group_label, &self.buffer, model);
        }
    }

    pub fn push(&mut self, joint: Mat4) {
        self.buffer.push(joint);
    }
}

pub struct ModelBuffers {
    pub mice_marines: DynamicBuffer<ModelInstance>,
    pub mice_marines_joints: JointBuffer,

    pub mice_engineers: DynamicBuffer<ModelInstance>,
    pub mice_engineers_joints: JointBuffer,

    pub pumps: DynamicBuffer<ModelInstance>,
    pub pump_joints: JointBuffer,

    pub bullets: DynamicBuffer<ModelInstance>,
    pub command_indicators: DynamicBuffer<ModelInstance>,
    pub command_paths: DynamicBuffer<ModelInstance>,
    pub armouries: DynamicBuffer<ModelInstance>,
    pub cheese_droplets: DynamicBuffer<ModelInstance>,
    pub explosions: DynamicBuffer<ModelInstance>,

    pub building_plan: BuildingPlan,
}

impl ModelBuffers {
    pub fn new(context: &RenderContext, assets: &Assets) -> Self {
        Self {
            mice_marines: DynamicBuffer::new(
                &context.device,
                50,
                "Cheese mice marines instance buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            mice_marines_joints: JointBuffer::new(
                context,
                400,
                "Cheese mice marines joints buffer",
                "Cheese mice marines joints bind group",
                &assets.mouse_model,
            ),
            mice_engineers: DynamicBuffer::new(
                &context.device,
                50,
                "Cheese mice engineers instance buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            mice_engineers_joints: JointBuffer::new(
                context,
                400,
                "Cheese mice engineers joints buffer",
                "Cheese mice engineers joints bind group",
                &assets.mouse_model,
            ),
            pumps: DynamicBuffer::new(
                &context.device,
                10,
                "Cheese pumps buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            pump_joints: JointBuffer::new(
                context,
                20,
                "Cheese pump joints buffer",
                "Cheese pump joints bind group",
                &assets.pump_model,
            ),

            bullets: DynamicBuffer::new(
                &context.device,
                200,
                "Cheese bullet buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            command_indicators: DynamicBuffer::new(
                &context.device,
                20,
                "Cheese command indicators buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            command_paths: DynamicBuffer::new(
                &context.device,
                20,
                "Cheese command paths buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            armouries: DynamicBuffer::new(
                &context.device,
                10,
                "Cheese armoury buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            cheese_droplets: DynamicBuffer::new(
                &context.device,
                5000,
                "Cheese cheese droplets buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            explosions: DynamicBuffer::new(
                &context.device,
                1,
                "Cheese explosions buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            building_plan: BuildingPlan {
                building: None,
                buffer: StaticBuffer::new(
                    &context.device,
                    ModelInstance::default(),
                    "Cheese building plan buffer",
                    wgpu::BufferUsage::VERTEX,
                ),
            },
        }
    }

    pub fn upload(&mut self, context: &RenderContext, assets: &Assets) {
        self.bullets.upload(context);
        self.command_indicators.upload(context);
        self.command_paths.upload(context);
        self.armouries.upload(context);
        self.cheese_droplets.upload(context);
        self.pumps.upload(context);
        self.building_plan.upload(context);
        self.mice_marines.upload(context);
        self.mice_engineers.upload(context);
        self.explosions.upload(context);
        self.mice_marines_joints
            .upload(context, &assets.mouse_model);
        self.mice_engineers_joints
            .upload(context, &assets.mouse_model);
        self.pump_joints.upload(context, &assets.pump_model);
    }
}

pub struct TitlescreenBuffer {
    pub moon: StaticBuffer<ModelInstance>,
    pub stars: wgpu::Buffer,
    pub num_stars: u32,
}

impl TitlescreenBuffer {
    pub fn new<R: rand::Rng>(device: &wgpu::Device, rng: &mut R) -> Self {
        let stars = crate::titlescreen::create_stars(rng);

        Self {
            moon: StaticBuffer::new(
                device,
                ModelInstance {
                    flat_colour: Vec4::new(1.0, 1.0, 1.0, 1.0),
                    transform: Mat4::identity(),
                },
                "Cheese titlescreen moon buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            stars: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cheese titlescreen stars buffer"),
                contents: bytemuck::cast_slice(&stars),
                usage: wgpu::BufferUsage::VERTEX,
            }),
            num_stars: stars.len() as u32,
        }
    }

    pub fn upload(&self, context: &RenderContext) {
        self.moon.upload(context);
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct ModelInstance {
    pub flat_colour: Vec4,
    pub transform: Mat4,
}

impl Default for ModelInstance {
    fn default() -> Self {
        Self {
            transform: Mat4::identity(),
            flat_colour: Vec4::one(),
        }
    }
}
