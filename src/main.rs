mod renderer;

use winit::{
    event::{ElementState, Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
fn main() -> anyhow::Result<()> {
    futures::executor::block_on(run())
}

async fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new();

    let (mut renderer, _) = renderer::Renderer::new(&event_loop).await?;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        renderer.resize(size.width as u32, size.height as u32);
                    }
                    _ => {}
                }
            },
            Event::MainEventsCleared => renderer.request_redraw(),
            Event::RedrawRequested(_) => renderer.render(),
            _ => {}
        }
    });

    Ok(())
}
