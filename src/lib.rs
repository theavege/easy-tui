//~ https://ratatui.rs/concepts/application-patterns/the-elm-architecture/

pub mod prelude {
    pub use ratatui::{
        Frame,
        crossterm::event::{Event, KeyCode, KeyEventKind},
        layout::{Constraint, Layout, Rect},
        prelude::Stylize,
        text::Line,
        widgets::{Block, BorderType, Borders, Paragraph},
    };
    pub use std::error::Error;

    const TICK: std::time::Duration = std::time::Duration::from_nanos(1_000_000_000 / 48);

    fn config_path(title: &str) -> Result<std::path::PathBuf, Box<dyn Error>> {
        use std::io::{Error as IoError, ErrorKind};

        let name = std::path::Path::new(title);
        if title.trim().is_empty()
            || name.components().count() != 1
            || name
                .components()
                .next()
                .is_none_or(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "title must be a non-empty file name",
            )
            .into());
        }
        #[cfg(windows)]
        let root = std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(|home| std::path::PathBuf::from(home).join("AppData/Roaming"))
            })
            .ok_or_else(|| {
                IoError::new(
                    ErrorKind::NotFound,
                    "could not locate the user config directory",
                )
            })?;
        #[cfg(not(windows))]
        let root = std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
            })
            .ok_or_else(|| {
                IoError::new(
                    ErrorKind::NotFound,
                    "could not locate the user config directory",
                )
            })?;
        std::fs::create_dir_all(&root)?;
        Ok(root.join(name))
    }

    fn lifecycle<T: Console>(
        model: &mut T,
        path: &str,
        run: impl FnOnce(&mut T) -> Result<(), Box<dyn Error>>,
    ) -> Result<(), Box<dyn Error>> {
        let result = model.load(path).and_then(|()| run(model));
        model.exit(path)?;
        result
    }

    pub trait Console: Default {
        fn load(&mut self, path: &str) -> Result<(), Box<dyn Error>>;
        fn handle(&mut self, event: Event) -> bool;
        fn update(&mut self, dt: f32);
        fn draw(&self, frame: &mut Frame);
        fn exit(&self, path: &str) -> Result<(), Box<dyn Error>>;
        fn run(title: &str) -> Result<(), Box<dyn Error>> {
            let path = config_path(title)?;
            let path = path
                .to_str()
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "config path is not valid UTF-8",
                    )
                })?
                .to_owned();
            ratatui::run(|terminal| {
                let mut model = Self::default();
                lifecycle(&mut model, &path, |model| {
                    (|| {
                        if let Ok(size) = terminal.size()
                            && !model.handle(Event::Resize(size.width, size.height))
                        {
                            return Ok(());
                        }
                        let mut last = std::time::Instant::now();
                        loop {
                            let timeout = TICK.saturating_sub(last.elapsed());
                            if ratatui::crossterm::event::poll(timeout)? {
                                loop {
                                    if !model.handle(ratatui::crossterm::event::read()?) {
                                        return Ok(());
                                    }
                                    if !ratatui::crossterm::event::poll(std::time::Duration::ZERO)?
                                    {
                                        break;
                                    }
                                }
                            }
                            let now = std::time::Instant::now();
                            model.update(now.saturating_duration_since(last).as_secs_f32());
                            last = now;
                            terminal.draw(|frame| model.draw(frame))?;
                        }
                    })()
                })
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{Console, Event, Frame, config_path, lifecycle};
        use std::{cell::Cell, error::Error};

        #[derive(Default)]
        struct HookModel {
            load_fails: bool,
            exited: Cell<bool>,
        }

        impl Console for HookModel {
            fn load(&mut self, _: &str) -> Result<(), Box<dyn Error>> {
                if self.load_fails {
                    Err("load failed".into())
                } else {
                    Ok(())
                }
            }

            fn handle(&mut self, _: Event) -> bool {
                true
            }
            fn update(&mut self, _: f32) {}
            fn draw(&self, _: &mut Frame) {}

            fn exit(&self, _: &str) -> Result<(), Box<dyn Error>> {
                self.exited.set(true);
                Ok(())
            }
        }

        #[test]
        fn config_path_rejects_empty_or_non_filename_titles() {
            for title in ["", "  ", "../app", "folder/app", ".", ".."] {
                assert!(config_path(title).is_err(), "accepted {title:?}");
            }
        }

        #[test]
        fn exit_runs_even_when_load_fails() {
            let mut model = HookModel {
                load_fails: true,
                exited: Cell::new(false),
            };

            assert!(lifecycle(&mut model, "app", |_| Ok(())).is_err());
            assert!(model.exited.get());
        }
    }
}
