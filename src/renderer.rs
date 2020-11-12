use crate::resources::ScreenDimensions;
use std::sync::Arc;
use ultraviolet::{Mat4, Vec2, Vec3, Vec4};
use wgpu::util::DeviceExt;
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

mod lines_pipeline;
mod model_pipelines;
mod torus_pipeline;

pub use lines_pipeline::{LineBuffers, LinesPipeline};
pub use model_pipelines::{ModelBuffers, ModelInstance, ModelPipelines};
pub use torus_pipeline::{TorusBuffer, TorusInstance, TorusPipeline};

const DISPLAY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

// Shared items for rendering.
pub struct RenderContext {
    pub swap_chain: wgpu::SwapChain,
    pub window: Window,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    surface: wgpu::Surface,
    swap_chain_desc: wgpu::SwapChainDescriptor,
    pub depth_texture: wgpu::TextureView,
    sampler: wgpu::Sampler,

    perspective_buffer: wgpu::Buffer,
    view_buffer: wgpu::Buffer,
    main_bind_group_layout: wgpu::BindGroupLayout,
    main_bind_group: Arc<wgpu::BindGroup>,

    pub joint_bind_group_layout: wgpu::BindGroupLayout,
}

impl RenderContext {
    pub async fn new(event_loop: &EventLoop<()>) -> anyhow::Result<Self> {
        let window = WindowBuilder::new().build(event_loop)?;

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
        //self.line_renderer.resize(&self.queue, width, height);

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

    fn upload(&mut self, context: &RenderContext) {
        if self.waiting.is_empty() {
            self.len = 0;
            return;
        }

        let bytes = bytemuck::cast_slice(&self.waiting);

        if self.waiting.len() <= self.capacity {
            context.queue.write_buffer(&self.buffer, 0, bytes);
            self.len = self.waiting.len();
        } else {
            self.capacity = (self.capacity * 2).max(self.waiting.len());
            log::debug!("Resizing '{}' to {} items", self.label, self.capacity);
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

pub struct TextBuffer {
    pub glyph_brush: wgpu_glyph::GlyphBrush<(), wgpu_glyph::ab_glyph::FontRef<'static>>,
}

impl TextBuffer {
    pub fn new(device: &wgpu::Device) -> anyhow::Result<Self> {
        let font = wgpu_glyph::ab_glyph::FontRef::try_from_slice(include_bytes!(
            "../fonts/Roboto_Mono/RobotoMono-Bold.ttf"
        ))?;

        let glyph_brush =
            wgpu_glyph::GlyphBrushBuilder::using_font(font).build(&device, DISPLAY_FORMAT);

        Ok(Self { glyph_brush })
    }

    pub fn render_text(&mut self, screen_position: (f32, f32), text: &str, dpi_scaling: f32) {
        self.glyph_brush.queue(
            wgpu_glyph::Section::new()
                .with_screen_position(screen_position)
                .add_text(
                    wgpu_glyph::Text::new(text)
                        .with_color([1.0; 4])
                        .with_scale(24.0 * dpi_scaling),
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
