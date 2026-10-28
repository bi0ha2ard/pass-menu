use std::{num::NonZeroU32, rc::Rc};

use ab_glyph::{Font, FontRef, Glyph, ScaleFont, point};
use softbuffer::{Context, Surface};
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, OwnedDisplayHandle},
    keyboard::{KeyCode, PhysicalKey},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
    window::Window,
};

use crate::painting::{BG, TEXT, draw_text, draw_text_box, rect};

#[derive(Debug)]
enum AppState {
    Initial,
    Suspended {
        window: Rc<Window>,
    },
    Running {
        surface: Surface<OwnedDisplayHandle, Rc<Window>>,
    },
}

pub struct App {
    context: Context<OwnedDisplayHandle>,
    state: AppState,
    ctrl_down: bool,
    entry_buffer: String,

    pass_entries: Vec<crate::pass::Entry>,
    matcher: frizbee::Matcher,
    matcher_results: Vec<frizbee::Match>,
    exit_with_submit: bool,

    font: ab_glyph::FontRef<'static>,
}

enum EventResult {
    Exit,
    TextChanged,
    Submitted,
    Nothing,
}

const MAX_ENTRY_LEN: usize = 1024;
const FONT_SIZE: f32 = 48.0;

impl App {
    pub fn new(
        context: Context<OwnedDisplayHandle>,
        pass_entries: Vec<crate::pass::Entry>,
    ) -> Self {
        Self {
            context,
            state: AppState::Initial,
            ctrl_down: false,
            entry_buffer: String::with_capacity(MAX_ENTRY_LEN),
            pass_entries,
            exit_with_submit: false,
            matcher: frizbee::Matcher::new("", &frizbee::Config::default()),
            matcher_results: Vec::default(),

            font: FontRef::try_from_slice(include_bytes!(
                "/usr/share/fonts/truetype/DejaVuSans.ttf"
            ))
            .expect("font"),
        }
    }

    pub fn get_selected_entry(&self) -> Option<&crate::pass::Entry> {
        if !self.exit_with_submit {
            return None;
        }
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

    fn get_surface(&mut self) -> &mut Surface<OwnedDisplayHandle, Rc<Window>> {
        let AppState::Running { surface } = &mut self.state else {
            unreachable!("got window event while suspended");
        };
        surface
    }


    fn render_ui(&mut self) {
        println!("{} | {}", self.matcher_results.len(), self.entry_buffer);
        let AppState::Running { surface } = &mut self.state else {
            unreachable!("got window event while suspended");
        };
        let mut buffer = surface.buffer_mut().expect("buffer");

        buffer.fill(BG);

        let sf = self.font.as_scaled(FONT_SIZE);

        draw_text_box(&mut buffer, &sf, &self.entry_buffer, 10, 10, 500, TEXT, 0x00aa0000);

        buffer.present().expect("present");
    }
}

impl ApplicationHandler for App {
    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if let StartCause::Init = cause {
            self.state = AppState::Suspended {
                window: Rc::new(
                    event_loop
                        .create_window(
                            Window::default_attributes()
                                .with_title("pass-menu")
                                .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
                                .with_resizable(false),
                        )
                        .unwrap(),
                ),
            };
        }
    }

    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let AppState::Suspended { window } = &mut self.state else {
            unreachable!("got resumed event while not suspended")
        };

        let mut surface =
            Surface::new(&self.context, window.clone()).expect("failed to create surface");

        // TODO: https://github.com/rust-windowing/softbuffer/issues/106
        let size = window.inner_size();
        if let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
            // Resize surface
            surface.resize(width, height).unwrap();
        }

        self.state = AppState::Running { surface };
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Drop the surface.
        let AppState::Running { surface } = &mut self.state else {
            unreachable!("got suspended event while not running");
        };
        let window = surface.window().clone();
        self.state = AppState::Suspended { window };
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if self.get_surface().window().id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    // Resize
                    self.get_surface().resize(width, height).unwrap();
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_ui();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => match self.handle_keyboard(&event) {
                EventResult::Exit => event_loop.exit(),
                EventResult::Submitted => {
                    self.exit_with_submit = true;
                    event_loop.exit();
                }
                EventResult::TextChanged => {
                    // TODO: only run matcher on tab
                    self.run_matcher();
                    self.get_surface().window().request_redraw();
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
