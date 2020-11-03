use crate::assets::{load_texture, Model};
use crate::resources::ScreenDimensions;
use ultraviolet::{Mat4, Vec2, Vec3};
use wgpu::util::DeviceExt;
use winit::{event_loop::EventLoop, window::Window};

mod lines;

const DISPLAY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct Renderer {
    swap_chain: wgpu::SwapChain,
    window: Window,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    swap_chain_desc: wgpu::SwapChainDescriptor,
    depth_texture: wgpu::TextureView,
    identity_instance_buffer: wgpu::Buffer,

    model_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,

    perspective_buffer: wgpu::Buffer,
    view_buffer: wgpu::Buffer,
    main_bind_group: wgpu::BindGroup,

    imgui_platform: imgui_winit_support::WinitPlatform,
    imgui_renderer: imgui_wgpu::Renderer,

    line_renderer: lines::Renderer,

    surface_model: Model,
    mouse_box_model: Model,
    selection_indicator_model: Model,
    surface_texture: wgpu::BindGroup,
    box_colours_texture: wgpu::BindGroup,
    colours_texture: wgpu::BindGroup,
}

impl Renderer {
    pub async fn new(
        event_loop: &EventLoop<()>,
        imgui: &mut imgui::Context,
    ) -> anyhow::Result<(Self, InstanceBuffers, ScreenDimensions)> {
        let window = Window::new(event_loop)?;

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("request_adapter failed"))?;

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

        // Create bind groups

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
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                ],
            });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            label: Some("Cheese Sampler"),
            ..Default::default()
        });

        let window_size = window.inner_size();

        let perspective_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cheese perspective buffer"),
            contents: bytemuck::bytes_of(&create_perspective_mat4(
                window_size.width,
                window_size.height,
            )),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let view = Mat4::look_at(
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        let view_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cheese view buffer"),
            contents: bytemuck::bytes_of(&view),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
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
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("Cheese main bind group"),
        });

        // Create bind group for textures

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Cheese texture bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                    },
                    count: None,
                }],
            });

        // Load models

        let surface_model = Model::load(include_bytes!("../models/surface.obj"), &device)?;
        let mouse_box_model = Model::load(include_bytes!("../models/mouse_box.obj"), &device)?;
        let selection_indicator_model =
            Model::load(include_bytes!("../models/selection_indicator.obj"), &device)?;

        // Load textures

        let mut init_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Cheese init_encoder"),
        });

        let surface_texture = load_texture(
            include_bytes!("../textures/surface.png"),
            &texture_bind_group_layout,
            &device,
            &mut init_encoder,
        )?;

        let box_colours_texture = load_texture(
            include_bytes!("../textures/box_colours.png"),
            &texture_bind_group_layout,
            &device,
            &mut init_encoder,
        )?;

        let colours_texture = load_texture(
            include_bytes!("../textures/colours.png"),
            &texture_bind_group_layout,
            &device,
            &mut init_encoder,
        )?;

        queue.submit(Some(init_encoder.finish()));

        // Create the shaders and pipeline

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Cheese pipeline layout"),
            bind_group_layouts: &[&main_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let vs = wgpu::include_spirv!("../shaders/shader.vert.spv");
        let vs_module = device.create_shader_module(vs);

        let fs = wgpu::include_spirv!("../shaders/shader.frag.spv");
        let fs_module = device.create_shader_module(fs);

        let model_pipeline = create_render_pipeline(
            &device,
            &pipeline_layout,
            "Cheese model pipeline",
            wgpu::PrimitiveTopology::TriangleList,
            &vs_module,
            &fs_module,
        );

        let line_pipeline = create_render_pipeline(
            &device,
            &pipeline_layout,
            "Cheese line pipeline",
            wgpu::PrimitiveTopology::LineList,
            &vs_module,
            &fs_module,
        );

        // Create the swap chain.

        let swap_chain_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: DISPLAY_FORMAT,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);
        let depth_texture = create_depth_texture(&device, window_size.width, window_size.height);

        let identity_instance_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(&Instance {
                    transform: Mat4::identity(),
                    uv_x_offset: 0.0,
                }),
                usage: wgpu::BufferUsage::VERTEX,
            });

        let mut imgui_platform = imgui_winit_support::WinitPlatform::init(imgui);
        imgui_platform.attach_window(
            imgui.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );

        let imgui_renderer = imgui_wgpu::RendererConfig::new()
            .set_texture_format(DISPLAY_FORMAT)
            .set_depth_format(DEPTH_FORMAT)
            .set_sample_count(1)
            .build(imgui, &device, &queue);

        let (line_renderer, line_buffers) =
            lines::Renderer::new(&device, window_size.width, window_size.height);

        let instance_buffers = InstanceBuffers {
            mice: GpuBuffer::new(
                &device,
                50,
                "Cheese mice instance buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            selection_indicators: GpuBuffer::new(
                &device,
                50,
                "Cheese selection indicators buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            command_paths: GpuBuffer::new(
                &device,
                50,
                "Cheese command paths buffer",
                wgpu::BufferUsage::VERTEX,
            ),
            line_buffers,
        };

        Ok((
            Self {
                swap_chain,
                window,
                device,
                queue,
                main_bind_group,
                perspective_buffer,
                view_buffer,
                swap_chain_desc,
                depth_texture,
                identity_instance_buffer,
                surface,
                model_pipeline,
                line_pipeline,
                line_renderer,
                // Imgui
                imgui_platform,
                imgui_renderer,
                // Models
                surface_model,
                mouse_box_model,
                selection_indicator_model,
                // Textures
                surface_texture,
                box_colours_texture,
                colours_texture,
            },
            instance_buffers,
            ScreenDimensions {
                width: window_size.width,
                height: window_size.height,
            },
        ))
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.swap_chain_desc.width = width;
        self.swap_chain_desc.height = height;
        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.swap_chain_desc);
        self.depth_texture = create_depth_texture(&self.device, width, height);
        self.line_renderer.resize(&self.queue, width, height);

        self.queue.write_buffer(
            &self.perspective_buffer,
            0,
            bytemuck::bytes_of(&create_perspective_mat4(width, height)),
        );
    }

    pub fn render(&mut self, view: Mat4, instance_buffers: &mut InstanceBuffers, ui: imgui::Ui) {
        self.queue
            .write_buffer(&self.view_buffer, 0, bytemuck::bytes_of(&view));

        instance_buffers.mice.upload(&self.device, &self.queue);
        instance_buffers
            .selection_indicators
            .upload(&self.device, &self.queue);
        instance_buffers
            .command_paths
            .upload(&self.device, &self.queue);

        if let Ok(frame) = self.swap_chain.get_current_frame() {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Cheese render encoder"),
                });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.output.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.125,
                                b: 0.125,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: Some(
                        wgpu::RenderPassDepthStencilAttachmentDescriptor {
                            attachment: &self.depth_texture,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: true,
                            }),
                            stencil_ops: None,
                        },
                    ),
                });

                render_pass.set_pipeline(&self.model_pipeline);
                render_pass.set_bind_group(0, &self.main_bind_group, &[]);

                // Draw mice
                if let Some((slice, num)) = instance_buffers.mice.get() {
                    render_pass.set_bind_group(1, &self.box_colours_texture, &[]);
                    render_pass.set_vertex_buffer(0, self.mouse_box_model.buffer.slice(..));
                    render_pass.set_vertex_buffer(1, slice);
                    render_pass.draw(0..self.mouse_box_model.num_vertices, 0..num);
                }

                // Draw selection indicators
                if let Some((slice, num)) = instance_buffers.selection_indicators.get() {
                    render_pass.set_bind_group(1, &self.box_colours_texture, &[]);
                    render_pass
                        .set_vertex_buffer(0, self.selection_indicator_model.buffer.slice(..));
                    render_pass.set_vertex_buffer(1, slice);
                    render_pass.draw(0..self.selection_indicator_model.num_vertices, 0..num);
                }

                // Draw surface
                render_pass.set_bind_group(1, &self.surface_texture, &[]);
                render_pass.set_vertex_buffer(0, self.surface_model.buffer.slice(..));
                render_pass.set_vertex_buffer(1, self.identity_instance_buffer.slice(..));
                render_pass.draw(0..self.surface_model.num_vertices, 0..1);

                // Draw Command paths
                if let Some((slice, num)) = instance_buffers.command_paths.get() {
                    render_pass.set_pipeline(&self.line_pipeline);
                    render_pass.set_bind_group(1, &self.colours_texture, &[]);
                    render_pass.set_vertex_buffer(0, slice);
                    render_pass.set_vertex_buffer(1, self.identity_instance_buffer.slice(..));
                    render_pass.draw(0..num, 0..1);
                }

                // Draw UI

                self.line_renderer.render(
                    &mut render_pass,
                    &mut instance_buffers.line_buffers,
                    &self.device,
                    &self.queue,
                );

                self.imgui_renderer
                    .render(ui.render(), &self.queue, &self.device, &mut render_pass)
                    .expect("Rendering failed");
            }

            self.queue.submit(Some(encoder.finish()));
        }
    }

    pub fn prepare_imgui(&mut self, imgui: &mut imgui::Context) {
        self.imgui_platform
            .prepare_frame(imgui.io_mut(), &self.window)
            .expect("Failed to prepare frame");
    }

    pub fn copy_event_to_imgui(
        &mut self,
        event: &winit::event::Event<()>,
        imgui: &mut imgui::Context,
    ) {
        self.imgui_platform
            .handle_event(imgui.io_mut(), &self.window, event);
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

pub fn create_perspective_mat4(window_width: u32, window_height: u32) -> Mat4 {
    ultraviolet::projection::perspective_wgpu_dx(
        45.0,
        window_width as f32 / window_height as f32,
        0.1,
        1000.0,
    )
}

fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Cheese depth texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub struct GpuBuffer<T> {
    buffer: wgpu::Buffer,
    capacity: usize,
    len: usize,
    label: &'static str,
    waiting: Vec<T>,
    usage: wgpu::BufferUsage,
}

impl<T: bytemuck::Pod> GpuBuffer<T> {
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

    fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.waiting.is_empty() {
            self.len = 0;
            return;
        }

        let bytes = bytemuck::cast_slice(&self.waiting);

        if self.waiting.len() <= self.capacity {
            queue.write_buffer(&self.buffer, 0, bytes);
            self.len = self.waiting.len();
        } else {
            self.capacity = (self.capacity * 2).max(self.waiting.len());
            log::debug!("Resizing '{}' to {} items", self.label, self.capacity);
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
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
            self.len = self.waiting.len();
        }

        self.waiting.clear();
    }

    fn get(&self) -> Option<(wgpu::BufferSlice, u32)> {
        if self.len > 0 {
            let byte_len = (self.len * std::mem::size_of::<T>()) as u64;

            return Some((self.buffer.slice(..byte_len), self.len as u32));
        } else {
            None
        }
    }
}

fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    label: &str,
    primitives: wgpu::PrimitiveTopology,
    vs_module: &wgpu::ShaderModule,
    fs_module: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some(label),
		layout: Some(layout),
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
			index_format: wgpu::IndexFormat::Uint16,
			vertex_buffers: &[
				wgpu::VertexBufferDescriptor {
					stride: std::mem::size_of::<Vertex>() as u64,
					step_mode: wgpu::InputStepMode::Vertex,
					attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float3, 2 => Float2],
				},
				wgpu::VertexBufferDescriptor {
					stride: std::mem::size_of::<Instance>() as u64,
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

pub struct InstanceBuffers {
    pub mice: GpuBuffer<Instance>,
    pub selection_indicators: GpuBuffer<Instance>,
    pub command_paths: GpuBuffer<Vertex>,
    pub line_buffers: lines::LineBuffers,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct Instance {
    pub uv_x_offset: f32,
    pub transform: Mat4,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}
