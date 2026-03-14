//! Minimal test to verify ratatui-image rendering works.

use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use image::{DynamicImage, Rgba, RgbaImage};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui_image::picker::Picker;
use ratatui_image::{FilterType, Resize, StatefulImage};
use std::time::Duration;

fn main() -> Result<()> {
    color_eyre::install()?;

    // Create a test image (red/blue checkerboard)
    let mut img = RgbaImage::new(200, 200);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        if (x / 20 + y / 20) % 2 == 0 {
            *pixel = Rgba([255, 0, 0, 255]);
        } else {
            *pixel = Rgba([0, 0, 255, 255]);
        }
    }
    let dyn_image = DynamicImage::ImageRgba8(img);

    // Init terminal
    let mut terminal = ratatui::init();

    // Init picker (must be after alternate screen, before event reading)
    let mut picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());

    // Env var override: FIGIF_PROTOCOL=halfblocks|sixel|kitty|iterm2
    if let Ok(proto) = std::env::var("FIGIF_PROTOCOL") {
        use ratatui_image::picker::ProtocolType;
        match proto.as_str() {
            "halfblocks" => picker.set_protocol_type(ProtocolType::Halfblocks),
            "sixel" => picker.set_protocol_type(ProtocolType::Sixel),
            "kitty" => picker.set_protocol_type(ProtocolType::Kitty),
            "iterm2" => picker.set_protocol_type(ProtocolType::Iterm2),
            _ => {}
        }
    }
    eprintln!(
        "Protocol: {:?}, Caps: {:?}",
        picker.protocol_type(),
        picker.capabilities()
    );

    // Create stateful protocol
    let mut state = picker.new_resize_protocol(dyn_image);

    loop {
        terminal.draw(|f: &mut Frame| {
            let area = f.area();
            let info = format!(
                "Protocol: {:?} | Font: {:?} | Area: {}x{} | Press 'q' to quit",
                picker.protocol_type(),
                picker.font_size(),
                area.width,
                area.height,
            );
            let header = Rect::new(area.x, area.y, area.width, 1);
            let image_area = Rect::new(
                area.x,
                area.y + 1,
                area.width,
                area.height.saturating_sub(1),
            );

            f.render_widget(Paragraph::new(info), header);

            let image_widget =
                StatefulImage::new().resize(Resize::Scale(Some(FilterType::Nearest)));
            f.render_stateful_widget(image_widget, image_area, &mut state);
        })?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && key.code == KeyCode::Char('q')
        {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}
