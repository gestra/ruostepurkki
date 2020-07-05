mod document;
mod protocol;
mod certificates;
mod ui;

fn main() {
    if let Ok(mut ui) = ui::TextUI::init() {
        match ui.main_loop() {
            Ok(()) => {}
            Err(e) => { 
                drop(ui);
                println!("Shutting down due to error: {}", e);
            }
        }
    }
}
