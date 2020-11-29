use crate::resources::{Camera, ScreenDimensions, Settings};
use std::sync::Arc;
use ultraviolet::{Mat4, Vec2, Vec3, Vec4};
use wgpu::util::DeviceExt;
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

mod lines_3d_pipeline;
mod lines_pipeline;
mod model_pipelines;
mod shadow_pipeline;
mod torus_pipeline;

pub use lines_3d_pipeline::{Lines3dBuffer, Lines3dPipeline};
pub use lines_pipeline::{Image, LineBuffers, LinesPipeline};
pub use model_pipelines::{ModelBuffers, ModelInstance, ModelPipelines, TitlescreenBuffer};
pub use shadow_pipeline::ShadowPipeline;
pub use torus_pipeline::{TorusBuffer, TorusInstance, TorusPipeline};

const DISPLAY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;

const SUN_DIRECTION: Vec3 = Vec3::new(5.0, 10.0, 0.0);
const BLUR_SCALE: f32 = 2.0;
const BLUR_STRENGTH: f32 = 5.0;

// Shared items for rendering.
pub struct RenderContext {
    pub swap_chain: wgpu::SwapChain,
    pub window: Window,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    surface: wgpu::Surface,
    swap_chain_desc: wgpu::SwapChainDescriptor,
    pub depth_texture: wgpu::TextureView,

    framebuffer_bind_group_layout: wgpu::BindGroupLayout,
    pub framebuffer_bind_group: wgpu::BindGroup,
    pub framebuffer: wgpu::TextureView,
    pub framebuffer_sampler: wgpu::Sampler,
    pub screen_dimension_uniform_buffer: wgpu::Buffer,
    pub post_processing_pipeline: wgpu::RenderPipeline,

    pub bloombuffer: wgpu::TextureView,
    pub bloombuffer_after_vertical: wgpu::TextureView,
    bloom_blur_vert_buffer: wgpu::Buffer,
    bloom_blur_hori_buffer: wgpu::Buffer,
    pub bloom_first_pass_bind_group: wgpu::BindGroup,
    pub bloom_second_pass_bind_group: wgpu::BindGroup,
    bloom_bind_group_layout: wgpu::BindGroupLayout,
    pub bloom_blur_pipeline: wgpu::RenderPipeline,

    pub shadow_texture: wgpu::TextureView,

    sampler: wgpu::Sampler,

    perspective_buffer: wgpu::Buffer,
    view_buffer: wgpu::Buffer,
    main_bind_group_layout: wgpu::BindGroupLayout,
    main_bind_group: Arc<wgpu::BindGroup>,

    pub joint_bind_group_layout: wgpu::BindGroupLayout,

    pub vs_transparent_module: wgpu::ShaderModule,
    pub fs_transparent_module: wgpu::ShaderModule,

    pub identity_instance_buffer: Arc<wgpu::Buffer>,

    pub shadow_uniform_bind_group: Arc<wgpu::BindGroup>,
    pub shadow_uniform_bind_group_layout: wgpu::BindGroupLayout,
    shadow_uniform_buffer: wgpu::Buffer,

    pub darken_pipeline: wgpu::RenderPipeline,
}

impl RenderContext {
    pub async fn new(event_loop: &EventLoop<()>, settings: &Settings) -> anyhow::Result<Self> {
        let window = WindowBuilder::new()
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .with_title("Cheese (working title)")
            .build(event_loop)?;

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or_else(|| anyhow::anyhow!(
                "'request_adapter' failed. If you get this on linux, try installing the vulkan drivers for your gpu. \
                You can check that they're working properly by running `vulkaninfo` or `vkcube`."
            ))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    shader_validation: true,
                },
                None,
            )
            .await?;

        // Create samplers

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            label: Some("Cheese sampler"),
            ..Default::default()
        });

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            label: Some("Cheese shadow sampler"),
            ..Default::default()
        });

        let framebuffer_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Cheese framebuffer sampler"),
            ..Default::default()
        });

        // Create basic buffers

        let window_size = window.inner_size();

        let perspective_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cheese perspective buffer"),
            contents: bytemuck::bytes_of(&create_perspective_mat4(
                window_size.width,
                window_size.height,
            )),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let view_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cheese view buffer"),
            contents: bytemuck::bytes_of(&Mat4::look_at(Vec3::one(), Vec3::zero(), Vec3::unit_y())),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let sun_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cheese sun buffer"),
            contents: &bytemuck::bytes_of(&SUN_DIRECTION),
            usage: wgpu::BufferUsage::UNIFORM,
        });

        let screen_dimension_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cheese screen dimension uniform buffer"),
                contents: &bytemuck::bytes_of(&ScreenDimensionUniform::new(
                    window_size.width,
                    window_size.height,
                )),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            });

        // Create the shadow attachment

        let shadow_texture = create_texture(
            &device,
            "Cheese shadow texture",
            settings.shadow_resolution,
            settings.shadow_resolution,
            DEPTH_FORMAT,
            wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        );

        // Create the main bind group

        let main_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Cheese main bind group layout"),
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
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Float,
                        },
                        count: None,
                    },
                ],
            });

        let main_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &main_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(perspective_buffer.slice(..)),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(view_buffer.slice(..)),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(sun_buffer.slice(..)),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&shadow_texture),
                },
            ],
            label: Some("Cheese main bind group"),
        });

        // Post-processing

        let framebuffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Cheese framebuffer bind group layout"),
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
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                ],
            });

        let (framebuffer, framebuffer_bind_group) = create_framebuffer(
            &device,
            &framebuffer_bind_group_layout,
            &framebuffer_sampler,
            window_size.width,
            window_size.height,
        );

        let post_processing_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Cheese post-processing pipeline layout"),
                bind_group_layouts: &[&framebuffer_bind_group_layout],
                push_constant_ranges: &[],
            });

        let vs_full_screen_quad =
            wgpu::include_spirv!("../shaders/compiled/full_screen_quad.vert.spv");
        let vs_full_screen_quad_module = device.create_shader_module(vs_full_screen_quad);
        let fs_post_processing =
            wgpu::include_spirv!("../shaders/compiled/post_processing.frag.spv");
        let fs_post_processing_module = device.create_shader_module(fs_post_processing);

        let post_processing_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Cheese post-processing pipeline"),
                layout: Some(&post_processing_pipeline_layout),
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_full_screen_quad_module,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &fs_post_processing_module,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor::default()),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[colour_state_descriptor(false)],
                depth_stencil_state: None,
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: INDEX_FORMAT,
                    vertex_buffers: &[],
                },
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });

        // Re-usable bind group layouts, buffers and shader modules

        let joint_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Cheese joint bind group layout"),
                entries: &[
                    // Joint transforms.
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            min_binding_size: None,
                            readonly: true,
                        },
                        count: None,
                    },
                    // Num joints - used for instances
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let vs_transparent = wgpu::include_spirv!("../shaders/compiled/transparent.vert.spv");
        let vs_transparent_module = device.create_shader_module(vs_transparent);

        let fs_transparent = wgpu::include_spirv!("../shaders/compiled/transparent.frag.spv");
        let fs_transparent_module = device.create_shader_module(fs_transparent);

        let identity_instance_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cheese identity instance buffer"),
                contents: bytemuck::bytes_of(&ModelInstance::default()),
                usage: wgpu::BufferUsage::VERTEX,
            });

        // Shadows

        let shadow_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cheese shadow uniform buffer"),
            contents: bytemuck::bytes_of(&ShadowUniforms::new(
                Vec2::one(),
                Vec2::zero(),
                Vec2::zero(),
                Vec2::zero(),
            )),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let shadow_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Cheese shadow uniform bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let shadow_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cheese shadow uniform bind group"),
            layout: &shadow_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(shadow_uniform_buffer.slice(..)),
            }],
        });

        // Bloom

        let bloom_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Cheese bloom bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Float,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bloom_blur_vert_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cheese bloom blur vert buffer"),
            contents: bytemuck::bytes_of(&BloomBlurSettings {
                blur_scale: BLUR_SCALE,
                blur_strength: BLUR_STRENGTH,
                blur_direction: 0,
            }),
            usage: wgpu::BufferUsage::UNIFORM,
        });

        let bloom_blur_hori_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cheese bloom blur hori buffer"),
            contents: bytemuck::bytes_of(&BloomBlurSettings {
                blur_scale: BLUR_SCALE,
                blur_strength: BLUR_STRENGTH,
                blur_direction: 1,
            }),
            usage: wgpu::BufferUsage::UNIFORM,
        });

        let bloombuffer = create_texture(
            &device,
            "Cheese bloombuffer texture",
            window_size.width,
            window_size.height,
            DISPLAY_FORMAT,
            wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        );
        let bloombuffer_after_vertical = create_texture(
            &device,
            "Cheese bloombuffer after vert texture",
            window_size.width,
            window_size.height,
            DISPLAY_FORMAT,
            wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        );
        let bloom_first_pass_bind_group = create_bloom_blur_pass(
            &device,
            "Cheese bloom first pass bind group",
            &bloom_bind_group_layout,
            &framebuffer_sampler,
            &bloombuffer,
            &bloom_blur_vert_buffer,
        );
        let bloom_second_pass_bind_group = create_bloom_blur_pass(
            &device,
            "Cheese bloom second pass bind group",
            &bloom_bind_group_layout,
            &framebuffer_sampler,
            &bloombuffer_after_vertical,
            &bloom_blur_hori_buffer,
        );

        let fs_blur = wgpu::include_spirv!("../shaders/compiled/blur.frag.spv");
        let fs_blur_module = device.create_shader_module(fs_blur);

        let bloom_blur_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Cheese bloom blur pipeline layout"),
                bind_group_layouts: &[&bloom_bind_group_layout],
                push_constant_ranges: &[],
            });

        let bloom_blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Cheese bloom blur pipeline"),
            layout: Some(&bloom_blur_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_full_screen_quad_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_blur_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor::default()),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[additive_colour_state_descriptor()],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: INDEX_FORMAT,
                vertex_buffers: &[],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        // Darkening pipeline for menus on top of the game.

        let fs_darken = wgpu::include_spirv!("../shaders/compiled/darken.frag.spv");
        let fs_darken_module = device.create_shader_module(fs_darken);

        let darken_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Cheese darken pipeline layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let darken_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Cheese darken pipeline"),
            layout: Some(&darken_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_full_screen_quad_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_darken_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor::default()),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[colour_state_descriptor(true)],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: INDEX_FORMAT,
                vertex_buffers: &[],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        // Create the swap chain

        let swap_chain_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: DISPLAY_FORMAT,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);

        // Create the depth attachment

        let depth_texture = create_texture(
            &device,
            "Cheese depth texture",
            window_size.width,
            window_size.height,
            DEPTH_FORMAT,
            wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        );

        Ok(Self {
            swap_chain,
            window,
            device,
            queue,
            surface,
            swap_chain_desc,
            depth_texture,
            perspective_buffer,
            view_buffer,
            main_bind_group_layout,
            sampler,
            joint_bind_group_layout,
            main_bind_group: Arc::new(main_bind_group),
            fs_transparent_module,
            vs_transparent_module,
            framebuffer,
            framebuffer_bind_group,
            framebuffer_bind_group_layout,
            framebuffer_sampler,
            post_processing_pipeline,
            screen_dimension_uniform_buffer,
            shadow_texture,
            identity_instance_buffer: Arc::new(identity_instance_buffer),
            shadow_uniform_bind_group: Arc::new(shadow_uniform_bind_group),
            shadow_uniform_bind_group_layout,
            shadow_uniform_buffer,
            bloombuffer,
            bloombuffer_after_vertical,
            bloom_blur_vert_buffer,
            bloom_blur_hori_buffer,
            bloom_first_pass_bind_group,
            bloom_second_pass_bind_group,
            bloom_bind_group_layout,
            bloom_blur_pipeline,
            darken_pipeline,
        })
    }

    pub fn set_cursor_icon(&self, cursor_icon: winit::window::CursorIcon) {
        self.window.set_cursor_icon(cursor_icon);
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.swap_chain_desc.width = width;
        self.swap_chain_desc.height = height;
        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.swap_chain_desc);
        self.depth_texture = create_texture(
            &self.device,
            "Cheese depth texture",
            width,
            height,
            DEPTH_FORMAT,
            wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        );
        let (framebuffer, framebuffer_bind_group) = create_framebuffer(
            &self.device,
            &self.framebuffer_bind_group_layout,
            &self.framebuffer_sampler,
            width,
            height,
        );
        self.framebuffer = framebuffer;
        self.framebuffer_bind_group = framebuffer_bind_group;

        self.bloombuffer = create_texture(
            &self.device,
            "Cheese bloombuffer texture",
            width,
            height,
            DISPLAY_FORMAT,
            wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        );
        self.bloombuffer_after_vertical = create_texture(
            &self.device,
            "Cheese bloombuffer after vert texture",
            width,
            height,
            DISPLAY_FORMAT,
            wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        );
        self.bloom_first_pass_bind_group = create_bloom_blur_pass(
            &self.device,
            "Cheese bloom first pass bind group",
            &self.bloom_bind_group_layout,
            &self.framebuffer_sampler,
            &self.bloombuffer,
            &self.bloom_blur_vert_buffer,
        );
        self.bloom_second_pass_bind_group = create_bloom_blur_pass(
            &self.device,
            "Cheese bloom second pass bind group",
            &self.bloom_bind_group_layout,
            &self.framebuffer_sampler,
            &self.bloombuffer_after_vertical,
            &self.bloom_blur_hori_buffer,
        );

        self.queue.write_buffer(
            &self.screen_dimension_uniform_buffer,
            0,
            bytemuck::bytes_of(&ScreenDimensionUniform::new(width, height)),
        );

        self.queue.write_buffer(
            &self.perspective_buffer,
            0,
            bytemuck::bytes_of(&create_perspective_mat4(width, height)),
        );
    }

    pub fn update_view(&self, view: Mat4) {
        self.queue
            .write_buffer(&self.view_buffer, 0, bytemuck::bytes_of(&view));
    }

    pub fn update_from_camera(&self, camera: &Camera) {
        self.update_view(camera.to_matrix());

        let screen_dimensions = self.screen_dimensions();
        let top_left = camera.cast_ray(Vec2::new(0.0, 0.0), &screen_dimensions);
        let top_right = camera.cast_ray(
            Vec2::new(screen_dimensions.width as f32, 0.0),
            &screen_dimensions,
        );
        let bottom_right = camera.cast_ray(screen_dimensions.as_vec(), &screen_dimensions);

        self.queue.write_buffer(
            &self.shadow_uniform_buffer,
            0,
            bytemuck::bytes_of(&ShadowUniforms::new(
                camera.looking_at,
                top_left,
                top_right,
                bottom_right,
            )),
        );
    }

    pub fn screen_dimensions(&self) -> ScreenDimensions {
        let dimensions = self.window.inner_size();
        ScreenDimensions {
            width: dimensions.width,
            height: dimensions.height,
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn submit(&self, commands: wgpu::CommandBuffer) {
        self.queue.submit(Some(commands));
    }
}

pub fn create_perspective_mat4(window_width: u32, window_height: u32) -> Mat4 {
    ultraviolet::projection::perspective_wgpu_dx(
        45.0,
        window_width as f32 / window_height as f32,
        0.1,
        250.0,
    )
}

fn create_bloom_blur_pass(
    device: &wgpu::Device,
    label: &str,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    input: &wgpu::TextureView,
    direction: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(input),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(direction.slice(..)),
            },
        ],
    })
}

fn create_framebuffer(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    width: u32,
    height: u32,
) -> (wgpu::TextureView, wgpu::BindGroup) {
    let framebuffer = create_texture(
        device,
        "Cheese framebuffer texture",
        width,
        height,
        DISPLAY_FORMAT,
        wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
    );

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Cheese framebuffer bind group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&framebuffer),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });

    (framebuffer, bind_group)
}

fn create_texture(
    device: &wgpu::Device,
    label: &str,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsage,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn colour_state_descriptor(alpha_blend: bool) -> wgpu::ColorStateDescriptor {
    if alpha_blend {
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
    }
}

fn additive_colour_state_descriptor() -> wgpu::ColorStateDescriptor {
    wgpu::ColorStateDescriptor {
        format: DISPLAY_FORMAT,
        write_mask: wgpu::ColorWrite::ALL,
        color_blend: wgpu::BlendDescriptor {
            operation: wgpu::BlendOperation::Add,
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
        },
        alpha_blend: wgpu::BlendDescriptor {
            operation: wgpu::BlendOperation::Add,
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::DstAlpha,
        },
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct ScreenDimensionUniform {
    screen_dimensions: Vec2,
}

impl ScreenDimensionUniform {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            screen_dimensions: Vec2::new(width as f32, height as f32),
        }
    }
}

pub struct StaticBuffer<T: bytemuck::Pod> {
    buffer: wgpu::Buffer,
    contents: T,
}

impl<T: bytemuck::Pod> StaticBuffer<T> {
    fn new(device: &wgpu::Device, contents: T, label: &str, usage: wgpu::BufferUsage) -> Self {
        Self {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::bytes_of(&contents),
                usage: usage | wgpu::BufferUsage::COPY_DST,
            }),
            contents,
        }
    }

    pub fn write(&mut self, contents: T) {
        self.contents = contents;
    }

    fn upload(&self, context: &RenderContext) {
        context
            .queue
            .write_buffer(&self.buffer, 0, bytemuck::bytes_of(&self.contents));
    }
}

pub struct DynamicBuffer<T: bytemuck::Pod> {
    buffer: wgpu::Buffer,
    capacity: usize,
    len: usize,
    label: &'static str,
    waiting: Vec<T>,
    usage: wgpu::BufferUsage,
}

impl<T: bytemuck::Pod> DynamicBuffer<T> {
    fn new(
        device: &wgpu::Device,
        base_capacity: usize,
        label: &'static str,
        usage: wgpu::BufferUsage,
    ) -> Self {
        Self {
            capacity: base_capacity,
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (base_capacity * std::mem::size_of::<T>()) as u64,
                usage: usage | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: false,
            }),
            len: 0,
            label,
            waiting: Vec::with_capacity(base_capacity),
            usage,
        }
    }

    pub fn push(&mut self, item: T) {
        self.waiting.push(item)
    }

    // Upload the waiting buffer to the gpu. Returns whether the gpu buffer was resized.
    fn upload(&mut self, context: &RenderContext) -> bool {
        if self.waiting.is_empty() {
            self.len = 0;
            return false;
        }

        self.len = self.waiting.len();
        let bytes = bytemuck::cast_slice(&self.waiting);

        if self.waiting.len() <= self.capacity {
            context.queue.write_buffer(&self.buffer, 0, bytes);
            self.waiting.clear();
            false
        } else {
            self.capacity = (self.capacity * 2).max(self.waiting.len());
            log::debug!(
                "Resizing '{}' to {} items to fit {} items",
                self.label,
                self.capacity,
                self.len
            );
            self.buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: (self.capacity * std::mem::size_of::<T>()) as u64,
                usage: self.usage | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: true,
            });
            self.buffer
                .slice(..bytes.len() as u64)
                .get_mapped_range_mut()
                .copy_from_slice(bytes);
            self.buffer.unmap();
            self.waiting.clear();
            true
        }
    }

    fn get(&self) -> Option<(wgpu::BufferSlice, u32)> {
        if self.len > 0 {
            let byte_len = (self.len * std::mem::size_of::<T>()) as u64;

            Some((self.buffer.slice(..byte_len), self.len as u32))
        } else {
            None
        }
    }

    pub fn len_waiting(&self) -> usize {
        self.waiting.len()
    }
}

pub struct TextBuffer {
    pub glyph_brush: wgpu_glyph::GlyphBrush<(), wgpu_glyph::ab_glyph::FontRef<'static>>,
}

pub enum Font {
    Ui = 0,
    Title = 1,
}

impl Font {
    pub fn scale(&self) -> f32 {
        match self {
            Self::Ui => 24.0,
            Self::Title => 64.0,
        }
    }
}

pub enum TextAlignment {
    Default,
    Center,
    HorizontalRight,
}

impl TextBuffer {
    pub fn new(device: &wgpu::Device) -> anyhow::Result<Self> {
        let fonts = vec![
            wgpu_glyph::ab_glyph::FontRef::try_from_slice(include_bytes!(
                "../fonts/Roboto_Mono/RobotoMono-Bold.ttf"
            ))?,
            wgpu_glyph::ab_glyph::FontRef::try_from_slice(include_bytes!(
                "../fonts/Chewy/Chewy-Regular.ttf"
            ))?,
        ];

        let glyph_brush =
            wgpu_glyph::GlyphBrushBuilder::using_fonts(fonts).build(&device, DISPLAY_FORMAT);

        Ok(Self { glyph_brush })
    }

    pub fn render_text(
        &mut self,
        screen_position: Vec2,
        text: &str,
        font: Font,
        scale_multiplier: f32,
        dpi_scaling: f32,
        alignment: TextAlignment,
        colour: Vec4,
    ) {
        let layout = match alignment {
            TextAlignment::Default => wgpu_glyph::Layout::default(),
            TextAlignment::Center => wgpu_glyph::Layout::default()
                .h_align(wgpu_glyph::HorizontalAlign::Center)
                .v_align(wgpu_glyph::VerticalAlign::Center),
            TextAlignment::HorizontalRight => {
                wgpu_glyph::Layout::default().h_align(wgpu_glyph::HorizontalAlign::Right)
            }
        };

        let scale = font.scale();
        let id = font as usize;
        let colour: [f32; 4] = colour.into();

        self.glyph_brush.queue(
            wgpu_glyph::Section::new()
                .with_screen_position((screen_position.x, screen_position.y))
                .with_layout(layout)
                .add_text(
                    wgpu_glyph::Text::new(text)
                        .with_color(colour)
                        .with_font_id(wgpu_glyph::FontId(id))
                        .with_scale(scale * scale_multiplier * dpi_scaling),
                ),
        );
    }
}

use crate::assets::Model;

pub fn draw_model<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    model: &'a Model,
    instances: wgpu::BufferSlice<'a>,
    num_instances: u32,
) {
    render_pass.set_vertex_buffer(0, model.vertices.slice(..));
    render_pass.set_vertex_buffer(1, instances);
    render_pass.set_index_buffer(model.indices.slice(..));
    render_pass.draw_indexed(0..model.num_indices, 0, 0..num_instances);
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct AnimatedVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub joints: Vec4,
    pub joint_weights: Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ShadowUniforms {
    light_projection_view: Mat4,
}

impl ShadowUniforms {
    fn new(look_at: Vec2, top_left: Vec2, top_right: Vec2, bottom_right: Vec2) -> Self {
        // Use the corner points of the camera view to figure out good corners for the projection
        // matrix.
        let top_left = top_left - look_at;
        let top_right = top_right - look_at;
        let bottom_right = bottom_right - look_at;
        // multiply the sun direction by a 10 so that we can view shadows from a greater distance.
        // todo: this is hacky and doesn't produce great looking shadows on the pumps. It's probably
        // a better solution to lower this down and not show shadows from above a certain height.
        let sun_direction_multiplied = SUN_DIRECTION * 10.0;

        // Using the camera distance from the ground is just a guesstimate that seems to work well here.
        let near_plane = 0.1;
        let far_plane = (sun_direction_multiplied * 1.5).mag();

        let projection = ultraviolet::projection::orthographic_wgpu_dx(
            -bottom_right.y,
            -top_left.y,
            -top_right.x,
            -top_left.x,
            near_plane,
            far_plane,
        );

        let look_at = Vec3::new(look_at.x, 0.0, look_at.y);

        let view = Mat4::look_at(sun_direction_multiplied + look_at, look_at, Vec3::unit_y());

        Self {
            light_projection_view: projection * view,
        }
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct BloomBlurSettings {
    blur_scale: f32,
    blur_strength: f32,
    blur_direction: i32,
}
