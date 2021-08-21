use gear::{Engine, event::{Event, WindowEvent}};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    Engine::new().await.run(move |engine, event| {
        match event {
            Event::UpdateEvent { delta_time: _ } => {
                engine.renderer
                    .set_clear_color([0.03, 0.03, 0.03, 1.0])
                    .submit();
            },
            Event::TerminateEvent => (),
            Event::WindowEvent(event) => match event {
                WindowEvent::Resized(_size) => {},
            },
            _ => (),
        }
    });
}
