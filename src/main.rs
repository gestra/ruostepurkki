mod gemini;
mod certificates;
mod ui;
extern crate mime;

fn main() {
    if let Ok(mut ui) = ui::TextUI::init() {
        ui.main_loop();
    }
}
