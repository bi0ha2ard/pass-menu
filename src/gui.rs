use winit::{
    application::ApplicationHandler,
    event::KeyEvent,
    keyboard::{KeyCode, PhysicalKey},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
    window::Window,
};

pub struct App {
    window: Option<Window>,
    ctrl_down: bool,
    entry_buffer: String,

    pass_entries: Vec<crate::pass::Entry>,
    matcher: frizbee::Matcher,
    matcher_results: Vec<frizbee::Match>,
}

enum EventResult {
    Exit,
    TextChanged,
    Submitted,
    Nothing,
}

const MAX_ENTRY_LEN: usize = 1024;

impl App {
    pub fn new(pass_entries: Vec<crate::pass::Entry>) -> Self {
        Self {
            window: None,
            ctrl_down: false,
            entry_buffer: String::with_capacity(MAX_ENTRY_LEN),
            pass_entries,
            matcher: frizbee::Matcher::new("", &frizbee::Config::default()),
            matcher_results: Vec::default(),
        }
    }

    pub fn get_selected_entry(&self) -> Option<&crate::pass::Entry> {
        // TODO: if user didn't tab, return current entry buffer instead
        self.matcher_results
            .first()
            .and_then(|hit| self.pass_entries.get(hit.index as usize))
    }

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
                if self.ctrl_down && !self.entry_buffer.is_empty() {
                    self.entry_buffer.clear();
                    return EventResult::TextChanged;
                }
                if self.entry_buffer.pop().is_some() {
                    return EventResult::TextChanged;
                }
                return EventResult::Nothing;
            }
            PhysicalKey::Code(KeyCode::Enter) => return EventResult::Submitted,
            _ => {}
        }
        if let Some(t) = key_event.text.as_ref() {
            if self.entry_buffer.len() >= MAX_ENTRY_LEN {
                return EventResult::Nothing;
            }
            self.entry_buffer.push_str(t.as_str());
            return EventResult::TextChanged;
        }
        EventResult::Nothing
    }

    fn run_matcher(&mut self) {
        if self.entry_buffer.is_empty() {
            self.matcher_results.clear();
            return;
        }

        self.matcher.set_pattern(self.entry_buffer.clone());
        self.matcher_results = self.matcher.match_list(&self.pass_entries);
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
            winit::event::WindowEvent::RedrawRequested => {
                println!("{} | {}", self.matcher_results.len(), self.entry_buffer);
            }
            winit::event::WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => match self.handle_keyboard(&event) {
                EventResult::Exit | EventResult::Submitted => event_loop.exit(),
                EventResult::TextChanged => {
                    // TODO: only run matcher on tab
                    self.run_matcher();
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
