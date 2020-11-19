use crate::resources::ScreenDimensions;
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
mod torus_pipeline;

pub use lines_3d_pipeline::{Lines3dBuffer, Lines3dPipeline};
pub use lines_pipeline::{LineBuffers, LinesPipeline};
pub use model_pipelines::{ModelBuffers, ModelInstance, ModelPipelines, TitlescreenBuffer};
pub use torus_pipeline::{TorusBuffer, TorusInstance, TorusPipeline};

const DISPLAY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;

const SUN_DIRECTION: Vec3 = Vec3::new(5.0, 10.0, 0.0);

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
    pub post_processing_pipeline: wgpu::RenderPipeline,

    sampler: wgpu::Sampler,

    perspective_buffer: wgpu::Buffer,
    view_buffer: wgpu::Buffer,
    main_bind_group_layout: wgpu::BindGroupLayout,
    main_bind_group: Arc<wgpu::BindGroup>,

    pub joint_bind_group_layout: wgpu::BindGroupLayout,

    pub fs_transparent_module: wgpu::ShaderModule,
}

impl RenderContext {
    pub async fn new(event_loop: &EventLoop<()>) -> anyhow::Result<Self> {
        let window = WindowBuilder::new()
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
                        visibility: wgpu::ShaderStage::VERTEX,
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

        let sun_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cheese sun buffer"),
            contents: &bytemuck::bytes_of(&SUN_DIRECTION),
            usage: wgpu::BufferUsage::UNIFORM,
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
            ],
            label: Some("Cheese main bind group"),
        });

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

        let post_processing_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Cheese post-processing pipeline layout"),
                bind_group_layouts: &[&framebuffer_bind_group_layout],
                push_constant_ranges: &[],
            });

        let vs_post_processing =
            wgpu::include_spirv!("../shaders/compiled/post_processing.vert.spv");
        let vs_post_processing_module = device.create_shader_module(vs_post_processing);
        let fs_post_processing =
            wgpu::include_spirv!("../shaders/compiled/post_processing.frag.spv");
        let fs_post_processing_module = device.create_shader_module(fs_post_processing);

        let post_processing_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Cheese post-processing pipeline"),
                layout: Some(&post_processing_pipeline_layout),
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_post_processing_module,
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

        let (framebuffer, framebuffer_bind_group) = create_framebuffer(
            &device,
            &framebuffer_bind_group_layout,
            &sampler,
            window_size.width,
            window_size.height,
        );

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

        let fs_transparent = wgpu::include_spirv!("../shaders/compiled/transparent.frag.spv");
        let fs_transparent_module = device.create_shader_module(fs_transparent);

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
            framebuffer,
            framebuffer_bind_group,
            framebuffer_bind_group_layout,
            post_processing_pipeline,
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
        self.depth_texture = create_depth_texture(&self.device, width, height);
        let (framebuffer, framebuffer_bind_group) = create_framebuffer(
            &self.device,
            &self.framebuffer_bind_group_layout,
            &self.sampler,
            width,
            height,
        );
        self.framebuffer = framebuffer;
        self.framebuffer_bind_group = framebuffer_bind_group;

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
        1000.0,
    )
}

fn create_framebuffer(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    width: u32,
    height: u32,
) -> (wgpu::TextureView, wgpu::BindGroup) {
    let framebuffer = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Cheese framebuffer texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DISPLAY_FORMAT,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        })
        .create_view(&wgpu::TextureViewDescriptor::default());

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
            Self::Title => 48.0,
        }
    }
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
        center: bool,
        colour: Vec4,
    ) {
        let layout = if center {
            wgpu_glyph::Layout::default()
                .h_align(wgpu_glyph::HorizontalAlign::Center)
                .v_align(wgpu_glyph::VerticalAlign::Center)
        } else {
            wgpu_glyph::Layout::default()
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
