mod animation;
mod assets;
mod ecs;
mod pathfinding;
mod renderer;
mod resources;
mod scenarios;
mod titlescreen;

use crate::assets::Assets;
use crate::renderer::{
    LineBuffers, Lines3dBuffer, Lines3dPipeline, LinesPipeline, ModelBuffers, ModelPipelines,
    RenderContext, ShadowPipeline, TextBuffer, TitlescreenBuffer, TorusBuffer, TorusPipeline,
};
use crate::resources::{
    Camera, CameraControls, CheeseCoins, ControlGroups, CursorIcon, DebugControls, DeltaTime,
    DpiScaling, Gravity, Keypress, Keypresses, Mode, MouseState, PlayerSide, RayCastLocation,
    RtsControls, ScreenDimensions, SelectedUnitsAbilities,
};
use legion::*;
use rand::SeedableRng;
use ultraviolet::{Mat4, Vec2, Vec3};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

fn main() -> anyhow::Result<()> {
    futures::executor::block_on(run())
}

async fn run() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new();

    let mut rng = rand::rngs::SmallRng::from_entropy();

    let mut render_context = RenderContext::new(&event_loop).await?;
    let (assets, animations, command_buffer) = Assets::new(&render_context.device())?;
    render_context.submit(command_buffer);
    let model_pipelines = ModelPipelines::new(&render_context, &assets);
    let torus_pipeline = TorusPipeline::new(&render_context);
    let lines_pipeline = LinesPipeline::new(&render_context, &assets);
    let lines_3d_pipeline = Lines3dPipeline::new(&render_context);
    let shadow_pipeline = ShadowPipeline::new(&render_context);
    let model_buffers = ModelBuffers::new(&render_context, &assets);
    let torus_buffer = TorusBuffer::new(render_context.device());
    let lines_buffers = LineBuffers::new(render_context.device());
    let text_buffer = TextBuffer::new(render_context.device())?;
    let lines_3d_buffer = Lines3dBuffer::new(render_context.device());
    let titlescreen_buffer = TitlescreenBuffer::new(render_context.device(), &mut rng);

    let mut world = World::default();
    let mut resources = Resources::default();
    resources.insert(model_buffers);
    resources.insert(torus_buffer);
    resources.insert(lines_buffers);
    resources.insert(text_buffer);
    resources.insert(lines_3d_buffer);
    resources.insert(titlescreen_buffer);
    resources.insert(render_context.screen_dimensions());
    resources.insert(CameraControls::default());
    resources.insert(Camera::default());
    resources.insert(MouseState::new(&render_context.screen_dimensions()));
    resources.insert(RtsControls::default());
    resources.insert(RayCastLocation::default());
    resources.insert(PlayerSide(ecs::Side::Green));
    resources.insert(ControlGroups::default());
    resources.insert(titlescreen::TitlescreenMoon::default());
    resources.insert(Mode::Playing);
    resources.insert(DebugControls::default());
    resources.insert(Gravity(5.0));
    resources.insert(CheeseCoins(100));
    resources.insert(SelectedUnitsAbilities::default());
    resources.insert(Keypresses::default());
    // Dpi scale factors are wierd. One of my laptops has it set at 1.33 and the other has it at 2.0.
    // Scaling things like selection boxes by 1.33 looks bad because one side can take up 1 pixel
    // and the other can take up 2 pixels. So I guess the best solution is to just round the value
    // idk.
    resources.insert(DpiScaling(
        render_context.window.scale_factor().round() as f32
    ));

    let mut map = pathfinding::Map::new();

    scenarios::one(&mut world, &animations, &mut map, &mut rng);

    resources.insert(animations);
    resources.insert(map);
    resources.insert(rng);

    let mut titlescreen_schedule = titlescreen::titlescreen_schedule();

    let mut builder = Schedule::builder();
    ecs::add_gameplay_systems(&mut builder);
    ecs::add_rendering_systems(&mut builder);
    let mut schedule = builder.build();

    let mut time = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    render_context.resize(size.width as u32, size.height as u32);
                    resources.insert(ScreenDimensions {
                        width: size.width as u32,
                        height: size.height as u32,
                    })
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode: code,
                            scancode,
                            ..
                        },
                    ..
                } => {
                    let pressed = *state == ElementState::Pressed;
                    let mut keypresses = resources.get_mut::<Keypresses>().unwrap();
                    keypresses.0.push(Keypress {
                        code: *code,
                        scancode: *scancode,
                        pressed,
                    });
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let mut camera_controls = resources.get_mut::<CameraControls>().unwrap();

                    camera_controls.zoom_delta += match delta {
                        MouseScrollDelta::LineDelta(_, y) => y * 200.0,
                        MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => *y as f32,
                    };
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let mut mouse_state = resources.get_mut::<MouseState>().unwrap();
                    mouse_state.position = Vec2::new(position.x as f32, position.y as f32);
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let pressed = *state == ElementState::Pressed;

                    let mut mouse_state = resources.get_mut::<MouseState>().unwrap();
                    let position = mouse_state.position;
                    match button {
                        MouseButton::Left => mouse_state.left_state.handle(position, pressed),
                        MouseButton::Right => mouse_state.right_state.handle(position, pressed),
                        _ => {}
                    }
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                let now = std::time::Instant::now();
                let elapsed = (now - time).as_secs_f32();
                time = now;
                resources.insert(DeltaTime(elapsed));
                resources.insert(CursorIcon(winit::window::CursorIcon::default()));

                let mode = *resources.get::<Mode>().unwrap();

                match mode {
                    Mode::Playing => schedule.execute(&mut world, &mut resources),
                    Mode::Titlescreen => titlescreen_schedule.execute(&mut world, &mut resources),
                    Mode::Quit => *control_flow = ControlFlow::Exit,
                }

                let cursor_icon = resources.get::<CursorIcon>().unwrap();
                render_context.set_cursor_icon(cursor_icon.0);
                render_context.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let mut model_buffers = resources.get_mut::<ModelBuffers>().unwrap();
                let mut torus_buffer = resources.get_mut::<TorusBuffer>().unwrap();
                let mut line_buffers = resources.get_mut::<LineBuffers>().unwrap();
                let mut text_buffer = resources.get_mut::<TextBuffer>().unwrap();
                let mut lines_3d_buffer = resources.get_mut::<Lines3dBuffer>().unwrap();
                let titlescreen_buffer = resources.get::<TitlescreenBuffer>().unwrap();
                let mode = *resources.get::<Mode>().unwrap();

                // Upload buffers to the gpu.

                match mode {
                    Mode::Playing => {
                        let camera = resources.get::<Camera>().unwrap();
                        render_context.update_from_camera(&camera);
                    }
                    Mode::Titlescreen => {
                        render_context.update_view(Mat4::look_at(
                            Vec3::zero(),
                            titlescreen::MOON_POSITION,
                            Vec3::new(0.0, 1.0, 0.0),
                        ));
                    }
                    _ => {}
                }

                model_buffers.upload(&render_context, &assets);
                torus_buffer.upload(&render_context);
                line_buffers.upload(&render_context);
                lines_3d_buffer.upload(&render_context);
                titlescreen_buffer.upload(&render_context);

                if let Ok(frame) = render_context.swap_chain.get_current_frame() {
                    let mut encoder = render_context.device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("Cheese render encoder"),
                        },
                    );

                    // Shadow pass

                    let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[],
                        depth_stencil_attachment: Some(
                            wgpu::RenderPassDepthStencilAttachmentDescriptor {
                                attachment: &render_context.shadow_texture,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(1.0),
                                    store: true,
                                }),
                                stencil_ops: None,
                            },
                        ),
                    });

                    if let Mode::Playing = mode {
                        render_shadows(&mut shadow_pass, &shadow_pipeline, &model_buffers, &assets);
                    }

                    drop(shadow_pass);

                    // Main rendering pass

                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[
                            wgpu::RenderPassColorAttachmentDescriptor {
                                attachment: &render_context.framebuffer,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color {
                                        r: 0.0,
                                        g: 0.0,
                                        b: 0.0,
                                        a: 1.0,
                                    }),
                                    store: true,
                                },
                            },
                            wgpu::RenderPassColorAttachmentDescriptor {
                                attachment: &render_context.bloombuffer,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color {
                                        r: 0.0,
                                        g: 0.0,
                                        b: 0.0,
                                        a: 1.0,
                                    }),
                                    store: true,
                                },
                            },
                        ],
                        depth_stencil_attachment: Some(
                            wgpu::RenderPassDepthStencilAttachmentDescriptor {
                                attachment: &render_context.depth_texture,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(1.0),
                                    store: true,
                                }),
                                stencil_ops: None,
                            },
                        ),
                    });

                    match mode {
                        Mode::Playing => {
                            render_playing(
                                &mut render_pass,
                                &model_pipelines,
                                &model_buffers,
                                &torus_pipeline,
                                &torus_buffer,
                                &lines_pipeline,
                                &line_buffers,
                                &lines_3d_pipeline,
                                &lines_3d_buffer,
                                &assets,
                            );
                        }
                        Mode::Titlescreen => {
                            model_pipelines.render_single_with_transform(
                                &mut render_pass,
                                &assets.cheese_moon_model,
                                &assets.surface_texture,
                                &titlescreen_buffer.moon,
                            );
                            model_pipelines.render_transparent_buffer(
                                &mut render_pass,
                                &assets.billboard_model,
                                &titlescreen_buffer.stars,
                                titlescreen_buffer.num_stars,
                            );
                            lines_pipeline.render(&mut render_pass, &line_buffers, &assets);
                        }
                        Mode::Quit => {}
                    }

                    drop(render_pass);

                    // First bloom pass
                    // todo: setting to disable bloom

                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &render_context.bloombuffer_after_vertical,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.0,
                                    g: 0.0,
                                    b: 0.0,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });

                    render_pass.set_pipeline(&render_context.bloom_blur_pipeline);
                    render_pass.set_bind_group(0, &render_context.bloom_first_pass_bind_group, &[]);
                    render_pass.draw(0..3, 0..1);

                    drop(render_pass);

                    // Second bloom pass and composit onto framebuffer

                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &render_context.framebuffer,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });

                    render_pass.set_pipeline(&render_context.bloom_blur_pipeline);
                    render_pass.set_bind_group(
                        0,
                        &render_context.bloom_second_pass_bind_group,
                        &[],
                    );
                    render_pass.draw(0..3, 0..1);

                    drop(render_pass);

                    // Post-processing pass

                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.output.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.0,
                                    g: 0.0,
                                    b: 0.0,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });

                    render_pass.set_pipeline(&render_context.post_processing_pipeline);
                    render_pass.set_bind_group(0, &render_context.framebuffer_bind_group, &[]);
                    render_pass.draw(0..3, 0..1);

                    drop(render_pass);

                    // Text rendering pass

                    let size = render_context.window.inner_size();
                    let mut staging_belt = wgpu::util::StagingBelt::new(10);

                    // Now render all the text to a seperate render pass.
                    text_buffer
                        .glyph_brush
                        .draw_queued(
                            &render_context.device,
                            &mut staging_belt,
                            &mut encoder,
                            &frame.output.view,
                            size.width,
                            size.height,
                        )
                        .unwrap();

                    staging_belt.finish();

                    // Do I need to do this?
                    // staging_belt.recall();

                    render_context.queue.submit(Some(encoder.finish()));
                }
            }
            _ => {}
        }
    });
}

fn render_shadows<'a>(
    shadow_pass: &mut wgpu::RenderPass<'a>,
    shadow_pipeline: &'a ShadowPipeline,
    model_buffers: &'a ModelBuffers,
    assets: &'a Assets,
) {
    shadow_pipeline.render_static(shadow_pass, &assets.armoury_model, &model_buffers.armouries);
    shadow_pipeline.render_animated(
        shadow_pass,
        &assets.pump_model,
        &model_buffers.pump_joints.bind_group,
        &model_buffers.pumps,
    );
    shadow_pipeline.render_single(shadow_pass, &assets.surface_model);
    shadow_pipeline.render_static(
        shadow_pass,
        &assets.cheese_droplet_model,
        &model_buffers.cheese_droplets,
    );
    // Marines
    shadow_pipeline.render_animated(
        shadow_pass,
        &assets.mouse_model,
        &model_buffers.mice_marines_joints.bind_group,
        &model_buffers.mice_marines,
    );
    shadow_pipeline.render_animated(
        shadow_pass,
        &assets.blaster_model,
        &model_buffers.mice_marines_joints.bind_group,
        &model_buffers.mice_marines,
    );
    // Engineers
    shadow_pipeline.render_animated(
        shadow_pass,
        &assets.mouse_model,
        &model_buffers.mice_engineers_joints.bind_group,
        &model_buffers.mice_engineers,
    );
    shadow_pipeline.render_animated(
        shadow_pass,
        &assets.wrench_model,
        &model_buffers.mice_engineers_joints.bind_group,
        &model_buffers.mice_engineers,
    );
}

fn render_playing<'a>(
    mut render_pass: &mut wgpu::RenderPass<'a>,
    model_pipelines: &'a ModelPipelines,
    model_buffers: &'a ModelBuffers,
    torus_pipeline: &'a TorusPipeline,
    torus_buffer: &'a TorusBuffer,
    lines_pipeline: &'a LinesPipeline,
    line_buffers: &'a LineBuffers,
    lines_3d_pipeline: &'a Lines3dPipeline,
    lines_3d_buffer: &'a Lines3dBuffer,
    assets: &'a Assets,
) {
    // Render a bunch of models.
    model_pipelines.render_instanced(
        &mut render_pass,
        &model_buffers.armouries,
        &assets.armoury_texture,
        &assets.armoury_model,
    );
    model_pipelines.render_animated(
        &mut render_pass,
        &model_buffers.pumps,
        &assets.pump_texture,
        &assets.pump_model,
        &model_buffers.pump_joints.bind_group,
    );
    model_pipelines.render_instanced(
        &mut render_pass,
        &model_buffers.cheese_droplets,
        &assets.surface_texture,
        &assets.cheese_droplet_model,
    );
    // Mice marines
    model_pipelines.render_animated(
        &mut render_pass,
        &model_buffers.mice_marines,
        &assets.mouse_texture,
        &assets.mouse_model,
        &model_buffers.mice_marines_joints.bind_group,
    );
    model_pipelines.render_animated(
        &mut render_pass,
        &model_buffers.mice_marines,
        &assets.blaster_texture,
        &assets.blaster_model,
        &model_buffers.mice_marines_joints.bind_group,
    );
    // Mice engineers
    model_pipelines.render_animated(
        &mut render_pass,
        &model_buffers.mice_engineers,
        &assets.mouse_texture,
        &assets.mouse_model,
        &model_buffers.mice_engineers_joints.bind_group,
    );
    model_pipelines.render_animated(
        &mut render_pass,
        &model_buffers.mice_engineers,
        &assets.armoury_texture,
        &assets.wrench_model,
        &model_buffers.mice_engineers_joints.bind_group,
    );
    // Bullets
    model_pipelines.render_transparent_textured_with_bloom(
        &mut render_pass,
        &model_buffers.bullets,
        &assets.misc_texture,
        &assets.bullet_model,
    );
    torus_pipeline.render(&mut render_pass, &torus_buffer.toruses, &assets.torus_model);
    lines_3d_pipeline.render(&mut render_pass, &lines_3d_buffer.lines);
    model_pipelines.render_single(
        &mut render_pass,
        &assets.surface_texture,
        &assets.surface_model,
    );
    model_pipelines.render_transparent_textured_without_depth(
        &mut render_pass,
        &model_buffers.command_paths,
        &assets.misc_texture,
        &assets.command_path_model,
    );
    model_pipelines.render_transparent_textured_without_depth(
        &mut render_pass,
        &model_buffers.command_indicators,
        &assets.misc_texture,
        &assets.command_indicator_model,
    );
    // Helmets
    model_pipelines.render_transparent_animated(
        &mut render_pass,
        &model_buffers.mice_marines,
        &assets.mouse_texture,
        &assets.mouse_helmet_model,
        &model_buffers.mice_marines_joints.bind_group,
    );
    model_pipelines.render_transparent_animated(
        &mut render_pass,
        &model_buffers.mice_engineers,
        &assets.mouse_texture,
        &assets.mouse_helmet_model,
        &model_buffers.mice_engineers_joints.bind_group,
    );

    if let Some((building, buffer)) = model_buffers.building_plan.get() {
        model_pipelines.render_transparent_buffer(
            &mut render_pass,
            match building {
                ecs::Building::Pump => &assets.pump_static_model,
                ecs::Building::Armoury => &assets.armoury_model,
            },
            buffer,
            1,
        );
    }

    // Render 2D items.
    lines_pipeline.render(&mut render_pass, &line_buffers, &assets);
}
