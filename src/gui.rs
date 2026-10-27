use winit::{
    application::ApplicationHandler,
    event::KeyEvent,
    keyboard::{KeyCode, PhysicalKey},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
    window::Window,
};

#[derive(Default)]
pub struct App {
    window: Option<Window>,
    ctrl_down: bool,
    entry_buffer: String,
}

enum EventResult {
    Exit,
    TextChanged,
    Nothing,
}

impl App {
    fn handle_keyboard(&mut self, key_event: &KeyEvent) -> EventResult {
        if !key_event.state.is_pressed() {
            return EventResult::Nothing;
        }
        if self.ctrl_down && key_event.key_without_modifiers() == "q" {
            return EventResult::Exit;
        }
        match key_event.physical_key {
            PhysicalKey::Code(KeyCode::Escape) => return EventResult::Exit,
            PhysicalKey::Code(KeyCode::Backspace) => {
                if self.entry_buffer.pop().is_some() {
                    return EventResult::TextChanged;
                }
                return EventResult::Nothing;
            }
            _ => {}
        }
        if let Some(t) = key_event.text.as_ref() {
            self.entry_buffer.push_str(t.as_str());
            return EventResult::TextChanged;
        }
        EventResult::Nothing
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("pass-menu")
                        .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
                        .with_resizable(false),
                )
                .unwrap(),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::RedrawRequested => {}
            winit::event::WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => match self.handle_keyboard(&event) {
                EventResult::Exit => event_loop.exit(),
                EventResult::TextChanged => {
                    println!("{}", self.entry_buffer);
                    self.window.as_ref().unwrap().request_redraw();
                }
                EventResult::Nothing => {}
            },
            winit::event::WindowEvent::ModifiersChanged(modifiers) => {
                self.ctrl_down = modifiers.state().control_key();
            }
            _ => (),
        }
    }
}
