mod assets;
mod ecs;
mod renderer;
mod resources;
mod animation;

use crate::assets::Assets;
use crate::renderer::{
    LineBuffers, LinesPipeline, ModelBuffers, ModelPipelines, RenderContext, TextBuffer,
    TorusBuffer, TorusPipeline,
};
use crate::resources::{
    Camera, CameraControls, CommandMode, CursorIcon, DeltaTime, DpiScaling, MouseState, PlayerSide,
    RayCastLocation, RtsControls, ScreenDimensions,
};
use legion::*;
use ultraviolet::{Vec2, Vec3};
use winit::{
    dpi::PhysicalPosition,
    event::{
        ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
};

fn main() -> anyhow::Result<()> {
    futures::executor::block_on(run())
}

fn add_gameplay_systems(builder: &mut legion::systems::Builder) {
    builder
        //.add_system(ecs::cast_ray_system())
        //.add_system(ecs::stop_attacks_on_dead_entities_system())
        .add_system(ecs::control_camera_system());
        /*.add_system(ecs::handle_left_click_system())
        .add_system(ecs::handle_right_click_system())
        .add_system(ecs::handle_stop_command_system())
        .add_system(ecs::handle_drag_selection_system())
        .add_system(ecs::set_move_to_system())
        .add_system(ecs::set_move_to_for_bullets_system())
        .add_system(ecs::avoidance_system())
        .add_system(ecs::add_attack_commands_system())
        .add_system(ecs::reduce_cooldowns_system())
        .flush()
        .add_system(ecs::move_units_system())
        .add_system(ecs::apply_steering_system())
        .add_system(ecs::firing_system())
        .add_system(ecs::apply_bullets_system())
        .flush()
        .add_system(ecs::handle_damaged_system());*/
}

async fn run() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new();

    let mut render_context = RenderContext::new(&event_loop).await?;
    let (mut assets, command_buffer, mut skin, mut animations, mut nodes, trans) = Assets::new(&render_context.device())?;
    render_context.submit(command_buffer);
    let model_pipelines = ModelPipelines::new(&render_context, &assets);
    let torus_pipeline = TorusPipeline::new(&render_context);
    let lines_pipeline = LinesPipeline::new(&render_context, &assets);
    let model_buffers = ModelBuffers::new(render_context.device());
    let torus_buffer = TorusBuffer::new(render_context.device());
    let lines_buffers = LineBuffers::new(render_context.device());
    let text_buffer = TextBuffer::new(render_context.device())?;

    let mut world = World::default();
    let mut resources = Resources::default();
    resources.insert(model_buffers);
    resources.insert(torus_buffer);
    resources.insert(lines_buffers);
    resources.insert(text_buffer);
    resources.insert(render_context.screen_dimensions());
    resources.insert(CameraControls::default());
    resources.insert(Camera {
        position: Vec3::new(0.0, 7.5, 10.0),
        looking_at: Vec3::new(0.0, 0.0, -10.0),
    });
    resources.insert(MouseState::new(&render_context.screen_dimensions()));
    resources.insert(RtsControls::default());
    resources.insert(RayCastLocation::default());
    resources.insert(PlayerSide(ecs::Side::Green));
    // Dpi scale factors are wierd. One of my laptops has it set at 1.33 and the other has it at 2.0.
    // Scaling things like selection boxes by 1.33 looks bad because one side can take up 1 pixel
    // and the other can take up 2 pixels. So I guess the best solution is to just round the value
    // idk.
    resources.insert(DpiScaling(
        render_context.window.scale_factor().round() as f32
    ));

    for i in 0..10 {
        ecs::Unit::MouseMarine.add_to_world(
            &mut world,
            Vec2::new(-10.0, i as f32 / 100.0),
            ecs::Facing(1.0),
            ecs::Side::Purple,
        );
    }

    for i in 0..10 {
        ecs::Unit::MouseMarine.add_to_world(
            &mut world,
            Vec2::new(10.0, i as f32 / 100.0),
            ecs::Facing(1.0),
            ecs::Side::Green,
        );
    }

    let mut builder = Schedule::builder();
    add_gameplay_systems(&mut builder);

    let mut schedule = builder
        /*
        // Rendering
        .add_system(ecs::render_bullets_system())
        .add_system(ecs::render_units_system())
        .add_system(ecs::render_selections_system())
        //.add_system(ecs::render_firing_ranges_system())
        .add_system(ecs::render_under_select_box_system())
        .add_system(ecs::render_drag_box_system())
        .add_system(ecs::render_command_paths_system())
        .add_system(ecs::render_ui_system())
        .add_system(ecs::render_health_bars_system())
        .add_system(ecs::render_unit_under_cursor_system())
        // Cleanup
        .flush()
        .add_system(ecs::update_mouse_buttons_system())
        */
        .build();

    let mut time = std::time::Instant::now();

    let mut T = 0.0;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { ref event, .. } => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        render_context.resize(size.width as u32, size.height as u32);
                        lines_pipeline.resize(
                            &render_context,
                            size.width as u32,
                            size.height as u32,
                        );
                        resources.insert(ScreenDimensions {
                            width: size.width as u32,
                            height: size.height as u32,
                        })
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state,
                                virtual_keycode: Some(code),
                                ..
                            },
                        ..
                    } => {
                        // Was previously disabled due to a bug where a right keypress gets
                        // inserted at the start. This doesn't seem to happen now as we start the
                        // window in fullscreen.

                        let pressed = *state == ElementState::Pressed;

                        let mut camera_controls = resources.get_mut::<CameraControls>().unwrap();
                        let mut rts_controls = resources.get_mut::<RtsControls>().unwrap();

                        handle_key(
                            code,
                            pressed,
                            &mut camera_controls,
                            &mut rts_controls,
                            control_flow,
                        );
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let mut camera_controls = resources.get_mut::<CameraControls>().unwrap();

                        camera_controls.zoom_delta += match delta {
                            MouseScrollDelta::LineDelta(_, y) => y * 100.0,
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
                }
            }
            Event::MainEventsCleared => {
                let now = std::time::Instant::now();
                let elapsed = (now - time).as_secs_f32();
                time = now;
                resources.insert(DeltaTime(elapsed));
                resources.insert(CursorIcon(winit::window::CursorIcon::default()));

                schedule.execute(&mut world, &mut resources);

                let cursor_icon = resources.get::<CursorIcon>().unwrap();
                render_context.set_cursor_icon(cursor_icon.0);
                render_context.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let camera = resources.get::<Camera>().unwrap();
                let mut model_buffers = resources.get_mut::<ModelBuffers>().unwrap();
                let mut torus_buffer = resources.get_mut::<TorusBuffer>().unwrap();
                let mut line_buffers = resources.get_mut::<LineBuffers>().unwrap();
                let mut text_buffer = resources.get_mut::<TextBuffer>().unwrap();

                animations.update(&mut nodes, 1.0 / 60.0);
                // I think this can just be an identity matrix.
                nodes.transform(Some(trans));
                nodes
                    .get_skins_transform()
                    .iter()
                    .for_each(|(index, transform)| {
                        skin.compute_joints_matrices(*transform, &nodes.nodes());
                    });
                use ultraviolet::Mat4;
                let mut matrices = vec![Mat4::identity(); skin.joints().len()];
                for (i, j) in skin.joints().iter().enumerate() {
                    let x: [[f32; 4]; 4] = j.matrix().into();
                    matrices[i] = x.into();
                }

                use wgpu::util::DeviceExt;
                
                let buffer = render_context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Cheese test joint buffer"),
                    contents: bytemuck::cast_slice(&matrices),
                    usage: wgpu::BufferUsage::STORAGE,
                });

                let joint_bind_group = render_context.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Cheese test joint bind group"),
                    layout: &render_context.joint_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
                    }],
                });

                T += 0.02;
                T = T % 1.0_f32;


                // Upload buffers to the gpu.
                render_context.update_view(camera.to_matrix());
                model_buffers.upload(&render_context);
                torus_buffer.upload(&render_context);
                line_buffers.upload(&render_context);

                if let Ok(frame) = render_context.swap_chain.get_current_frame() {
                    let mut encoder = render_context.device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("Cheese render encoder"),
                        },
                    );

                    // This is super messy and should be abstracted.
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
                                attachment: &render_context.depth_texture,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(1.0),
                                    store: true,
                                }),
                                stencil_ops: None,
                            },
                        ),
                    });

                    // Render a bunch of models.
                    /*model_pipelines.render_instanced(
                        &mut render_pass,
                        &model_buffers.mice,
                        &assets.mouse_texture,
                        &assets.mouse_model,
                    );
                    model_pipelines.render_instanced(
                        &mut render_pass,
                        &model_buffers.bullets,
                        &assets.colours_texture,
                        &assets.bullet_model,
                    );
                    torus_pipeline.render(
                        &mut render_pass,
                        &torus_buffer.toruses,
                        &assets.torus_model,
                    );
                    model_pipelines.render_lines(
                        &mut render_pass,
                        &model_buffers.command_paths,
                        &assets.colours_texture,
                    );*/
                    /*model_pipelines.render_single(
                        &mut render_pass,
                        &assets.surface_texture,
                        &assets.surface_model,
                    );*/
                    /*model_pipelines.render_transparent(
                        &mut render_pass,
                        &model_buffers.mice,
                        &assets.mouse_helmet_model,
                    );*/
                    model_pipelines.render_animated(
                        &mut render_pass,
                        &assets.character_texture,
                        &assets.gltf_model,
                        &joint_bind_group,
                    );

                    // Render 2D items.
                    //lines_pipeline.render(&mut render_pass, &line_buffers, &assets);

                    // We're done with this pass.
                    drop(render_pass);

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

fn handle_key(
    code: &VirtualKeyCode,
    pressed: bool,
    camera_controls: &mut CameraControls,
    rts_controls: &mut RtsControls,
    control_flow: &mut ControlFlow,
) {
    log::debug!("{:?} pressed: {}", code, pressed);

    match code {
        VirtualKeyCode::Up => camera_controls.up = pressed,
        VirtualKeyCode::Down => camera_controls.down = pressed,
        VirtualKeyCode::Left => camera_controls.left = pressed,
        VirtualKeyCode::Right => camera_controls.right = pressed,
        VirtualKeyCode::LShift => rts_controls.shift_held = pressed,
        VirtualKeyCode::S if pressed => rts_controls.stop_pressed = true,
        VirtualKeyCode::A if pressed => rts_controls.mode = CommandMode::AttackMove,
        VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,
        _ => {}
    }
}
