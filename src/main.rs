mod assets;
mod ecs;
mod renderer;
mod resources;

use crate::renderer::InstanceBuffers;
use crate::resources::{
    Camera, CameraControls, CommandMode, CursorIcon, DeltaTime, MouseState, PlayerSide,
    RayCastLocation, RtsControls, ScreenDimensions,
};
use legion::*;
use ultraviolet::{Vec2, Vec3};
use winit::{
    dpi::PhysicalPosition,
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta,
        VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
};

fn main() -> anyhow::Result<()> {
    futures::executor::block_on(run())
}

fn add_gameplay_systems(builder: &mut legion::systems::Builder) {
    builder
        .add_system(ecs::cast_ray_system())
        .add_system(ecs::stop_attacks_on_dead_entities_system())
        .add_system(ecs::control_camera_system())
        .add_system(ecs::handle_left_click_system())
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
        .add_system(ecs::handle_damaged_system());
}

async fn run() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new();

    let (mut renderer, instance_buffers, screen_dimensions) =
        renderer::Renderer::new(&event_loop).await?;

    let mut world = World::default();
    let mut resources = Resources::default();
    resources.insert(instance_buffers);
    resources.insert(screen_dimensions);
    resources.insert(CameraControls::default());
    resources.insert(Camera {
        position: Vec3::new(0.0, 20.0, 10.0),
        looking_at: Vec3::new(0.0, 0.0, 0.0),
    });
    resources.insert(MouseState::default());
    resources.insert(RtsControls::default());
    resources.insert(RayCastLocation::default());
    resources.insert(PlayerSide(ecs::Side::Purple));

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
        .build();

    let mut time = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { ref event, .. } => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        renderer.resize(size.width as u32, size.height as u32);
                        resources.insert(ScreenDimensions {
                            width: size.width as u32,
                            height: size.height as u32,
                        })
                    }
                    /*WindowEvent::KeyboardInput { input: KeyboardInput { state, virtual_keycode: Some(code), .. }, ..} => {
                        // Disabled due to a bug where a right keypress gets inserted at the start.

                        let pressed = *state == ElementState::Pressed;

                        let mut camera_controls = resources.get_mut::<CameraControls>().unwrap();
                        let mut rts_controls = resources.get_mut::<RtsControls>().unwrap();

                        handle_key(code, pressed, &mut camera_controls, &mut rts_controls);
                    },*/
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
            Event::DeviceEvent { ref event, .. } => match event {
                DeviceEvent::Key(KeyboardInput {
                    state,
                    virtual_keycode: Some(code),
                    ..
                }) => {
                    let pressed = *state == ElementState::Pressed;

                    let mut camera_controls = resources.get_mut::<CameraControls>().unwrap();
                    let mut rts_controls = resources.get_mut::<RtsControls>().unwrap();

                    handle_key(code, pressed, &mut camera_controls, &mut rts_controls);
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                let now = std::time::Instant::now();
                let elapsed = (now - time).as_secs_f32();
                time = now;
                resources.insert(DeltaTime(elapsed));
                resources.insert(CursorIcon(winit::window::CursorIcon::default()));

                schedule.execute(&mut world, &mut resources);

                let cursor_icon = resources.get::<CursorIcon>().unwrap();
                renderer.set_cursor_icon(cursor_icon.0);

                renderer.request_redraw()
            }
            Event::RedrawRequested(_) => {
                let mut instance_buffers = resources.get_mut::<InstanceBuffers>().unwrap();
                let camera = resources.get::<Camera>().unwrap();
                renderer.render(camera.to_matrix(), &mut instance_buffers)
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
        //VirtualKeyCode::Escape if pressed => rts_controls.mode = CommandMode::Normal,
        _ => {}
    }
}
