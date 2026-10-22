mod models {
    use rand::prelude::SliceRandom;
    use std::error::Error;

    #[derive(Default)]
    pub struct Score {
        pub typed: (u32, u32),
        pub highscore: (u32, u32),
        pub secs: f32,
        pub timer: f32,
        pub level: Level,
        pub lang: Lang,
    }

    impl Score {
        pub fn go(&mut self, dt: f32) -> bool {
            self.secs += dt;
            self.timer += dt;
            if self.timer >= (2.0 / self.level.to_u16() as f32) {
                self.timer = 0.0;
                true
            } else {
                false
            }
        }
        pub fn highscore(&mut self) {
            if self.typed > self.highscore {
                self.highscore = self.typed;
            }
        }
        pub fn wordlist(&self) -> Vec<String> {
            let mut result: Vec<String> = match (&self.lang, &self.level) {
                (&Lang::En, &Level::Easy) => include_str!("../assets/eng/A1.txt"),
                (&Lang::En, &Level::Normal) => include_str!("../assets/eng/A2.txt"),
                (&Lang::En, &Level::Hard) => include_str!("../assets/eng/B1.txt"),
                (&Lang::Su, &Level::Easy) => include_str!("../assets/suo/A1.txt"),
                (&Lang::Su, &Level::Normal) => include_str!("../assets/suo/A2.txt"),
                (&Lang::Su, &Level::Hard) => include_str!("../assets/suo/B1.txt"),
            }
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(str::to_string)
            .collect();
            result.shuffle(&mut rand::rng());
            result
        }

        pub fn draw(&self) -> String {
            let cpm = if self.secs > 0.0 {
                ((self.typed.0 as f32 / self.secs) * 60.0) as u32
            } else {
                0
            };
            let wpm = if self.secs > 0.0 {
                ((self.typed.1 as f32 / self.secs) * 60.0) as u32
            } else {
                0
            };
            format!(
                "Chars: {}; Words: {}; CPM: {}, WPM: {}",
                self.typed.0, self.typed.1, cpm, wpm,
            )
        }

        pub fn read(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
            let bytes = match std::fs::read(path) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error.into()),
            };
            if let Ok(text) = std::str::from_utf8(&bytes) {
                let mut parts = text.split_whitespace();
                if let (Some(chars), Some(words)) = (parts.next(), parts.next())
                    && let (Ok(chars), Ok(words)) = (chars.parse(), words.parse())
                {
                    self.highscore = (chars, words);
                    return Ok(());
                }
            }
            if bytes.len() >= 4 {
                self.highscore = (
                    bytes[0] as u32 * 255 + bytes[1] as u32,
                    bytes[2] as u32 * 255 + bytes[3] as u32,
                );
            }
            Ok(())
        }

        pub fn write(&self, path: &str) -> Result<(), Box<dyn Error>> {
            std::fs::write(path, format!("{} {}\n", self.highscore.0, self.highscore.1))?;
            Ok(())
        }
    }

    #[derive(Default)]
    pub enum Level {
        #[default]
        Easy = 0,
        Normal,
        Hard,
    }
    impl Level {
        pub fn to_str(&self) -> &str {
            match self {
                Self::Easy => "Easy",
                Self::Normal => "Normal",
                Self::Hard => "Hard",
            }
        }
        pub fn to_u16(&self) -> u16 {
            match self {
                Self::Easy => 2,
                Self::Normal => 3,
                Self::Hard => 4,
            }
        }
        pub fn switch(&self) -> Self {
            match self {
                Self::Easy => Self::Normal,
                Self::Normal => Self::Hard,
                Self::Hard => Self::Easy,
            }
        }
    }

    #[derive(Default)]
    pub enum Lang {
        #[default]
        En = 0,
        Su,
    }
    impl Lang {
        pub fn to_str(&self) -> &str {
            match self {
                Self::En => "English",
                Self::Su => "Finnish",
            }
        }
        pub fn switch(&self) -> Self {
            match self {
                Self::En => Self::Su,
                Self::Su => Self::En,
            }
        }
    }
}

use easy_tui::prelude::*;
use rand::RngExt;

#[derive(Default)]
pub enum Page {
    #[default]
    Menu = 0,
    Play,
}

#[derive(Default)]
pub struct World {
    page: Page,
    size: (u16, u16),
    store: models::Score,
    avatar: Vec<(String, String, u16, u16)>,
    npc: Vec<String>,
    banner: String,
}

impl World {
    fn banner() -> String {
        let Ok(font) = figleter::FIGfont::standard() else {
            return "TypeMaster".into();
        };
        font.convert("TypeMaster")
            .map(|fig| fig.to_string())
            .unwrap_or_else(|| "TypeMaster".into())
    }
    fn append(&mut self) -> bool {
        while let Some(line) = self.npc.pop() {
            let mut split = line.splitn(2, " - ");
            let word = split.next().unwrap_or("").trim();
            if word.is_empty() {
                continue;
            }
            let hint = split.next().unwrap_or("PLAY").trim().to_string();
            let width = word.chars().count() as u16;
            let max_x = self.size.0.saturating_sub(2 + width);
            let x = if max_x <= 2 {
                1
            } else {
                rand::rng().random_range(2..max_x)
            };
            self.avatar.push((word.to_string(), hint, x, 1));
            return true;
        }
        false
    }
    fn play(&mut self) {
        self.store.typed = (0, 0);
        self.store.secs = 0.0;
        self.store.timer = 0.0;
        self.avatar.clear();
        self.npc = self.store.wordlist();
        self.page = if self.append() {
            Page::Play
        } else {
            Page::Menu
        };
    }
    fn menu(&mut self) {
        if let Page::Play = self.page {
            self.store.highscore();
        }
        self.page = Page::Menu;
    }
    fn letter(&mut self, value: char) -> bool {
        if self.avatar.is_empty() {
            return false;
        }
        let Some(next) = self.avatar[0].0.chars().next() else {
            return false;
        };
        if value == next {
            self.store.typed.0 += 1;
            self.avatar[0].0.drain(..next.len_utf8());
            self.avatar[0].2 += 1;
            if self.avatar[0].0.is_empty() {
                self.store.typed.1 += 1;
                self.avatar.remove(0);
                if self.avatar.is_empty() && !self.append() {
                    self.store.highscore();
                    return false;
                }
            };
        };
        true
    }
}

impl Console for World {
    fn load(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        self.banner = Self::banner();
        self.store.read(path)?;
        self.npc = self.store.wordlist();
        Ok(())
    }
    fn update(&mut self, dt: f32) {
        if let Page::Play = self.page
            && self.store.go(dt)
            && !self.avatar.is_empty()
        {
            for word in &mut self.avatar {
                word.3 += 1;
            }
            if self.avatar[0].3 == self.size.1 / self.store.level.to_u16() {
                self.append();
            }
            if self.avatar[0].3 > (self.size.1 - 2) {
                self.store.highscore();
                self.page = Page::Menu;
            }
        }
    }
    fn handle(&mut self, event: Event) -> bool {
        match event {
            Event::FocusLost => self.menu(),
            Event::Resize(w, h) => {
                self.size = (w, h);
                if w < 5 || h < 3 {
                    self.menu();
                } else if let Page::Play = self.page {
                    for (word, _, x, y) in &mut self.avatar {
                        *x = (*x).min(w.saturating_sub(word.chars().count() as u16 + 3));
                        *y = (*y).min(h.saturating_sub(2));
                    }
                }
            }
            Event::Key(value) => match (&self.page, value.code, value.kind) {
                (&Page::Menu, KeyCode::Esc, KeyEventKind::Press) => return false,
                (&Page::Menu, KeyCode::Tab, KeyEventKind::Press) => {
                    self.store.lang = self.store.lang.switch();
                }
                (&Page::Menu, KeyCode::F(10), KeyEventKind::Press) => {
                    self.store.level = self.store.level.switch();
                }
                (&Page::Menu, KeyCode::Enter, KeyEventKind::Press) => self.play(),
                (&Page::Play, KeyCode::Char(value), KeyEventKind::Press) => {
                    if !self.letter(value) {
                        self.menu();
                    }
                }
                (&Page::Play, KeyCode::Esc, KeyEventKind::Press) => self.menu(),
                _ => {}
            },
            _ => {}
        }
        true
    }
    fn draw(&self, frame: &mut Frame) {
        match self.page {
            Page::Menu => {
                let outer = Block::bordered()
                    .yellow()
                    .border_type(BorderType::Rounded)
                    .title(
                        Line::from(vec![
                            "[".bold().yellow(),
                            "WELCOME".bold().reset(),
                            "]".bold().yellow(),
                        ])
                        .centered(),
                    )
                    .title_bottom(
                        Line::from(vec![
                            "[".bold().yellow(),
                            format!(
                                "Highest (Chars: {}, Words: {})",
                                self.store.highscore.0, self.store.highscore.1
                            )
                            .reset(),
                            "]".bold().yellow(),
                        ])
                        .centered(),
                    );
                let [title, body] =
                    Layout::vertical([Constraint::Ratio(1, 3); 2]).areas(outer.inner(frame.area()));
                let [_left, center, _right] = Layout::horizontal([
                    Constraint::Min(0),
                    Constraint::Length(45),
                    Constraint::Min(0),
                ])
                .areas(body);
                frame.render_widget(outer, frame.area());
                frame.render_widget(
                    Paragraph::new(self.banner.as_str()).green().centered(),
                    title,
                );
                frame.render_widget(
                    Paragraph::new(format!(
                        r#"
Press <ENTER> to start
      <F10>   to toggle difficulty: {}
      <TAB>   to toggle language:   {}
      <ESC>   to quit
"#,
                        self.store.level.to_str(),
                        self.store.lang.to_str(),
                    ))
                    .green()
                    .block(Block::default().borders(Borders::NONE)),
                    center,
                );
            }
            Page::Play => {
                let hint = self
                    .avatar
                    .first()
                    .map(|word| word.1.as_str())
                    .unwrap_or("PLAY");
                let outer = Block::bordered()
                    .yellow()
                    .border_type(BorderType::Rounded)
                    .title(
                        Line::from(vec![
                            "[".bold().yellow(),
                            hint.bold().green(),
                            "]".bold().yellow(),
                        ])
                        .centered(),
                    )
                    .title_bottom(
                        Line::from(vec![
                            "[".bold().yellow(),
                            self.store.draw().reset(),
                            "]".bold().yellow(),
                        ])
                        .centered(),
                    );
                frame.render_widget(outer, frame.area());
                for (word, _, x, y) in &self.avatar {
                    let mut chars = word.chars();
                    let Some(head) = chars.next() else {
                        continue;
                    };
                    frame.render_widget(
                        Paragraph::new(Line::from(vec![
                            "(".reset().bold(),
                            head.to_string().bold().yellow(),
                            chars.collect::<String>().reset().green(),
                            ")".reset().bold(),
                        ])),
                        Rect::new(*x, *y, word.chars().count() as u16 + 2, 1),
                    );
                }
            }
        };
    }
    fn exit(&self, path: &str) -> Result<(), Box<dyn Error>> {
        self.store.write(path)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    World::run("Words")
}
