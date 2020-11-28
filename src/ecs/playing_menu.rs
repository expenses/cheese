use crate::renderer::{Font, LineBuffers, TextAlignment, TextBuffer};
use crate::resources::{
    CursorIcon, DpiScaling, Keypress, Keypresses, Mode, MouseState, ScreenDimensions,
};
use crate::titlescreen::{point_in_area, selected_colour, text_selection_area, TEXT_COLOUR};
use ultraviolet::Vec2;
use winit::event::VirtualKeyCode;

const WIN_LOSE_MENU: &'static [(&'static str, Vec2)] =
    &[("Back to main menu", Vec2::new(0.5, 0.6))];

const PLAYING_MENU: &'static [(&'static str, Vec2)] = &[
    ("Continue", Vec2::new(0.5, 0.6)),
    ("Back to main menu", Vec2::new(0.5, 0.7)),
];

fn buttons(mode: &Mode) -> Option<&'static [(&'static str, Vec2)]> {
    match mode {
        Mode::ScenarioWon | Mode::ScenarioLost => Some(WIN_LOSE_MENU),
        Mode::PlayingMenu => Some(PLAYING_MENU),
        _ => None,
    }
}

#[legion::system]
pub fn handle_playing_menu_controls(
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] dpi_scaling: &DpiScaling,
    #[resource] mouse_state: &MouseState,
    #[resource] mode: &mut Mode,
    #[resource] keypresses: &mut Keypresses,
) {
    // Allow double-pressing escape to toggle the menu.
    for Keypress { code, pressed, .. } in keypresses.0.drain(..) {
        if let Some(code) = code {
            if pressed && code == VirtualKeyCode::Escape && *mode == Mode::PlayingMenu {
                *mode = Mode::Playing;
                return;
            }
        }
    }

    if !mouse_state.left_state.was_clicked() {
        return;
    }

    let screen_dimensions = screen_dimensions.as_vec();

    if let Some(buttons) = buttons(mode) {
        for &(text, position) in buttons {
            let center = position * screen_dimensions;
            let (top_left, bottom_right) = text_selection_area(center, text, dpi_scaling.0);
            if point_in_area(mouse_state.position, top_left, bottom_right) {
                match text {
                    "Continue" => *mode = Mode::Playing,
                    "Back to main menu" => *mode = Mode::Titlescreen,
                    _ => {}
                }
                return;
            }
        }
    }
}

#[legion::system]
pub fn render_playing_menu(
    #[resource] mode: &Mode,
    #[resource] text_buffer: &mut TextBuffer,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] dpi_scaling: &DpiScaling,
    #[resource] mouse_state: &MouseState,
    #[resource] cursor_icon: &mut CursorIcon,
) {
    let text = match mode {
        Mode::ScenarioWon => "Scenario Won",
        Mode::ScenarioLost => "Scenario Lost",
        Mode::PlayingMenu => "Playing Menu",
        _ => return,
    };

    let screen_dims = screen_dimensions.as_vec();

    text_buffer.render_text(
        Vec2::new(0.5, 0.4) * screen_dims,
        text,
        Font::Title,
        1.5,
        dpi_scaling.0,
        TextAlignment::Center,
        TEXT_COLOUR,
    );

    if let Some(buttons) = buttons(mode) {
        for &(text, position) in buttons {
            let center = position * screen_dims;
            let (top_left, bottom_right) = text_selection_area(center, text, dpi_scaling.0);
            let selected = point_in_area(mouse_state.position, top_left, bottom_right);

            if selected {
                cursor_icon.0 = winit::window::CursorIcon::Hand;
            }

            text_buffer.render_text(
                center,
                text,
                Font::Title,
                1.0,
                dpi_scaling.0,
                TextAlignment::Center,
                if selected {
                    selected_colour()
                } else {
                    TEXT_COLOUR
                },
            );
        }
    }
}

#[legion::system]
pub fn render_playing_menu_click_regions(
    #[resource] line_buffers: &mut LineBuffers,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] dpi_scaling: &DpiScaling,
    #[resource] mode: &Mode,
) {
    let screen_dimensions = screen_dimensions.as_vec();

    if let Some(buttons) = buttons(mode) {
        for &(text, position) in buttons {
            let center = position * screen_dimensions;
            let (top_left, bottom_right) = text_selection_area(center, text, dpi_scaling.0);
            line_buffers.draw_rect(top_left, bottom_right, dpi_scaling.0);
        }
    }
}
