use std::{num::NonZeroU32, rc::Rc};

use ab_glyph::{Font, FontRef};
use softbuffer::{Context, Surface};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{KeyEvent, StartCause, WindowEvent},
    event_loop::{self, ActiveEventLoop, EventLoop, OwnedDisplayHandle},
    keyboard::{KeyCode, PhysicalKey},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
    window::Window,
};

use crate::painting::{
    BG, BG_ALTERNATE, BG_PROMPT, FONT_SIZE, TEXT, TEXT_BOX_HEIGHT, draw_label, draw_text_box, hline,
};

const MAX_VISIBLE_SUGGESTIONS: usize = 7;

#[derive(Debug)]
enum AppState {
    Initial,
    Suspended {
        window: Rc<Window>,
    },
    Running {
        surface: Surface<OwnedDisplayHandle, Rc<Window>>,
    },
    Terminated,
}

impl AppState {
    fn close(&mut self) {
        drop(std::mem::replace(self, AppState::Terminated));
    }
}

pub struct App<'a, Candidate>
where
    Candidate: AsRef<str>,
{
    context: Context<OwnedDisplayHandle>,
    state: AppState,
    ctrl_down: bool,
    shift_down: bool,
    autocomplete_started: bool,
    entry_buffer: String,

    prompt: &'static str,
    entries: &'a [Candidate],
    matcher: frizbee::Matcher,
    matcher_results: Vec<frizbee::Match>,
    match_cursor: usize,
    exit_with_submit: bool,

    frame: usize,
    start_time: std::time::Instant,
    font: ab_glyph::FontRef<'static>,
}

pub enum AppResult<'a, Candidate>
where
    Candidate: AsRef<str>,
{
    Match(&'a Candidate),
    #[allow(unused)]
    Partial(String),
    Aborted,
}

enum EventResult {
    Exit,
    TextChanged,
    AutocompleteSelect(bool),
    Submitted,
    Nothing,
}

const WINDOW_WIDTH: u32 = 800;
const MAX_ENTRY_LEN: usize = 1024;
fn get_required_window_height(matches: usize) -> u32 {
    #[cfg(feature = "wayland")]
    {
        // TODO: hyprland doesn't resize the window for some reason, always show the lower part
        return (MAX_VISIBLE_SUGGESTIONS as u32 + 1) * TEXT_BOX_HEIGHT + 1;
    }
    let matches_height = (matches.min(MAX_VISIBLE_SUGGESTIONS) as u32) * TEXT_BOX_HEIGHT;
    matches_height + TEXT_BOX_HEIGHT + u32::from(matches > 0)
}

impl<'a, Candidate> App<'a, Candidate>
where
    Candidate: AsRef<str>,
{
    pub fn new(
        context: Context<OwnedDisplayHandle>,
        prompt: &'static str,
        entries: &'a [Candidate],
        immediate_autocomplete: bool,
    ) -> Self {
        Self {
            context,
            state: AppState::Initial,
            ctrl_down: false,
            shift_down: false,
            autocomplete_started: immediate_autocomplete,
            prompt,
            entry_buffer: String::with_capacity(MAX_ENTRY_LEN),
            entries,
            exit_with_submit: false,
            matcher: frizbee::Matcher::new("", &frizbee::Config::default()),
            matcher_results: Vec::default(),
            match_cursor: 0,

            frame: 0,
            start_time: std::time::Instant::now(),
            font: FontRef::try_from_slice(include_bytes!(env!(
                "CUSTOM_FONT_PATH",
                "font path must be set"
            )))
            .expect("font"),
        }
    }

    pub fn get_selected_entry(&self) -> AppResult<'a, Candidate> {
        if !self.exit_with_submit {
            return AppResult::Aborted;
        }
        if let Some(completed) = self
            .matcher_results
            .get(self.match_cursor)
            .and_then(|hit| self.entries.get(hit.index as usize))
        {
            return AppResult::Match(completed);
        }
        AppResult::Partial(self.entry_buffer.to_string())
    }

    fn backspace(&mut self, full_clear: bool) -> EventResult {
        if full_clear && !self.entry_buffer.is_empty() {
            self.entry_buffer.clear();
            return EventResult::TextChanged;
        }
        if self.entry_buffer.pop().is_some() {
            return EventResult::TextChanged;
        }
        EventResult::Nothing
    }

    fn handle_keyboard(&mut self, key_event: &KeyEvent) -> EventResult {
        // X11 keyboard event leak from launch hotkey
        #[cfg(feature = "x11")]
        if self.frame < 2 && self.start_time.elapsed() < std::time::Duration::from_millis(16) {
            return EventResult::Nothing;
        }
        if !key_event.state.is_pressed() {
            return EventResult::Nothing;
        }
        if self.ctrl_down && key_event.key_without_modifiers() == "q" {
            return EventResult::Exit;
        }
        if self.ctrl_down && key_event.key_without_modifiers() == "w" {
            return self.backspace(true);
        }
        if self.ctrl_down && key_event.key_without_modifiers() == "h" {
            return self.backspace(false);
        }
        match key_event.physical_key {
            PhysicalKey::Code(KeyCode::Escape) => return EventResult::Exit,
            PhysicalKey::Code(KeyCode::Backspace) => {
                return self.backspace(self.ctrl_down);
            }
            PhysicalKey::Code(KeyCode::Enter) => return EventResult::Submitted,
            PhysicalKey::Code(KeyCode::Tab) => {
                return EventResult::AutocompleteSelect(!self.shift_down);
            }
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

    fn gui_matcher_update(&mut self) {
        let prev_matches = self.matcher_results.len();
        self.run_matcher();
        let new_matches = self.matcher_results.len();
        if prev_matches != new_matches
            && !(prev_matches >= MAX_VISIBLE_SUGGESTIONS && new_matches >= MAX_VISIBLE_SUGGESTIONS)
        {
            let _ = self
                .get_surface()
                .window()
                .request_inner_size(PhysicalSize {
                    width: WINDOW_WIDTH,
                    height: get_required_window_height(new_matches),
                });
        }
        self.get_surface().window().request_redraw();
    }

    fn run_matcher(&mut self) {
        self.match_cursor = 0;
        if !self.autocomplete_started {
            return;
        }
        if self.entry_buffer.is_empty() {
            self.matcher_results.clear();
            return;
        }

        self.matcher.set_pattern(self.entry_buffer.clone());
        self.matcher_results = self.matcher.match_list(&self.entries);
    }

    fn get_surface(&mut self) -> &mut Surface<OwnedDisplayHandle, Rc<Window>> {
        let AppState::Running { surface } = &mut self.state else {
            unreachable!("got window event while suspended");
        };
        surface
    }

    fn render_ui(&mut self) {
        let AppState::Running { surface } = &mut self.state else {
            unreachable!("got window event while suspended");
        };
        let mut buffer = surface.buffer_mut().expect("buffer");

        buffer.fill(BG_ALTERNATE);

        let sf = self.font.as_scaled(FONT_SIZE as f32);

        let width = buffer.width().get();
        let height = buffer.height().get();

        let mut x = 0;
        let mut y = 0;
        x = draw_label(
            &mut buffer,
            &sf,
            self.prompt,
            x,
            y,
            WINDOW_WIDTH / 2,
            BG_ALTERNATE,
            Some(BG_PROMPT),
        );

        draw_text_box(
            &mut buffer,
            &sf,
            &self.entry_buffer,
            x,
            y,
            width.saturating_sub(x),
            TEXT,
            Some(BG),
            Some(BG_PROMPT),
        );

        y += TEXT_BOX_HEIGHT;
        if y < height && !self.matcher_results.is_empty() {
            hline(&mut buffer, 0, y, width, 0);
            y += 1;
        }
        x = 0;

        // Keep cursor in the middle
        let mut start = self
            .match_cursor
            .saturating_sub(MAX_VISIBLE_SUGGESTIONS / 2);
        const {
            assert!(MAX_VISIBLE_SUGGESTIONS % 2 == 1);
        }
        let end =
            (self.match_cursor + MAX_VISIBLE_SUGGESTIONS / 2 + 1).min(self.matcher_results.len());
        // once we reach the end of the list, move to the end
        if self.matcher_results.len() > MAX_VISIBLE_SUGGESTIONS
            && (end.saturating_sub(start)) < MAX_VISIBLE_SUGGESTIONS
        {
            start = end.saturating_sub(MAX_VISIBLE_SUGGESTIONS);
        }

        while y < height
            && let Some(entry) = self
                .matcher_results
                .get(start)
                .and_then(|e| self.entries.get(e.index as usize))
        {
            let x_offs = if start == self.match_cursor { 10 } else { 0 };
            draw_label(
                &mut buffer,
                &sf,
                entry.as_ref(),
                x + x_offs,
                y,
                width - x_offs,
                if start == self.match_cursor { TEXT } else { BG },
                if start == self.match_cursor {
                    Some(BG)
                } else {
                    None
                },
            );
            start += 1;
            y += TEXT_BOX_HEIGHT;
        }

        buffer.present().expect("present");
    }
}

impl<Candidate> ApplicationHandler for App<'_, Candidate>
where
    Candidate: AsRef<str>,
{
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
                                .with_resizable(false)
                                .with_inner_size(PhysicalSize {
                                    width: WINDOW_WIDTH,
                                    height: get_required_window_height(0),
                                }),
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
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.state.close();
            }
            WindowEvent::Destroyed => {
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
                self.frame += 1;
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => match self.handle_keyboard(&event) {
                EventResult::Exit => self.state.close(),
                EventResult::Submitted => {
                    self.exit_with_submit = true;
                    self.state.close();
                }
                EventResult::AutocompleteSelect(dir) => {
                    let was_autcompleting = self.autocomplete_started;
                    self.autocomplete_started = true;
                    if was_autcompleting {
                        let result_count = self.matcher_results.len();
                        if result_count > 0 {
                            self.match_cursor = (self.match_cursor
                                + if dir { 1 } else { result_count - 1 })
                            .rem_euclid(result_count);
                            self.get_surface().window().request_redraw();
                        }
                    } else {
                        self.gui_matcher_update();
                    }
                }
                EventResult::TextChanged => {
                    self.gui_matcher_update();
                }
                EventResult::Nothing => {}
            },

            winit::event::WindowEvent::ModifiersChanged(modifiers) => {
                self.ctrl_down = modifiers.state().control_key();
                self.shift_down = modifiers.state().shift_key();
            }
            _ => (),
        }
    }
}

pub fn run_app<'a, Candidate>(
    prompt: &'static str,
    entries: &'a [Candidate],
    immediate_autocomplete: bool,
) -> AppResult<'a, Candidate>
where
    Candidate: AsRef<str>,
{
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(event_loop::ControlFlow::Wait);
    let context = Context::new(event_loop.owned_display_handle()).expect("display handle");

    let mut app = App::new(context, prompt, &entries, immediate_autocomplete);

    event_loop.run_app(&mut app).expect("run");

    app.get_selected_entry()
}
