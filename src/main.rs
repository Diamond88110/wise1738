mod tui;
mod core;

fn main() -> std::io::Result<()> {
    tui::terminal::run()
}
