# easytui

A small application loop for [Ratatui](https://ratatui.rs/) terminal applications.

## Quick start

Implement the `Console` trait for a default-constructible application model, then call its `run` method. The crate is named `easy-tui` on crates.io and imported as `easy_tui` in Rust:

```rust,no_run
use easy_tui::prelude::*;

#[derive(Default)]
struct App;

impl Console for App {
    fn load(&mut self, _path: &str) -> Result<(), Box<dyn Error>> { Ok(()) }
    fn handle(&mut self, event: Event) -> bool {
        !matches!(event, Event::Key(key) if key.code == KeyCode::Esc && key.kind == KeyEventKind::Press)
    }
    fn update(&mut self, _dt: f32) {}
    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(Paragraph::new("Press Esc to quit"), frame.area());
    }
    fn exit(&self, _path: &str) -> Result<(), Box<dyn Error>> { Ok(()) }
}

fn main() -> Result<(), Box<dyn Error>> {
    App::run("counter")
}
```

## Lifecycle

`Console::run(title)` uses `title` as a single filename under the platform config directory and passes that full path to `load` and `exit`. It rejects empty titles and titles containing path components. `load` runs before the event loop; `handle` receives Ratatui/Crossterm events and returns `false` to stop; `update` receives elapsed seconds; `draw` renders each frame; and `exit` runs after `load` and the event loop, including when `load` returns an error. Hook errors are returned from `run`. On Windows the config root is `%APPDATA%`; on other platforms it follows `$XDG_CONFIG_HOME` or `$HOME/.config`.

See [examples/words.rs](examples/words.rs) for a complete typing game.
