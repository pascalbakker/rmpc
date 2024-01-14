use std::borrow::Cow;

use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::Rect, widgets::ListItem, Frame};
use strum::{Display, EnumIter, EnumVariantNames};

use crate::{
    mpd::{client::Client, commands::Song},
    state::State,
};

use super::{
    utils::dirstack::{DirStack, DirStackItem},
    KeyHandleResultInternal, SharedUiState,
};

pub mod albums;
pub mod artists;
pub mod directories;
pub mod logs;
pub mod playlists;
pub mod queue;

#[derive(Debug, Display, EnumVariantNames, Default, Clone, Copy, EnumIter, PartialEq)]
pub enum Screens {
    #[default]
    Queue,
    #[cfg(debug_assertions)]
    Logs,
    Directories,
    Artists,
    Albums,
    Playlists,
}

pub(super) trait Screen {
    type Actions;
    fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        app: &mut crate::state::State,
        shared_state: &mut SharedUiState,
    ) -> Result<()>;

    /// For any cleanup operations, ran when the screen hides
    fn on_hide(
        &mut self,
        _client: &mut Client<'_>,
        _app: &mut crate::state::State,
        _shared_state: &mut SharedUiState,
    ) -> Result<()> {
        Ok(())
    }

    /// For work that needs to be done BEFORE the first render
    fn before_show(
        &mut self,
        _client: &mut Client<'_>,
        _app: &mut crate::state::State,
        _shared: &mut SharedUiState,
    ) -> Result<()> {
        Ok(())
    }

    /// Used to keep the current state but refresh data
    fn refresh(
        &mut self,
        _client: &mut Client<'_>,
        _app: &mut crate::state::State,
        _shared: &mut SharedUiState,
    ) -> Result<()> {
        Ok(())
    }

    fn handle_action(
        &mut self,
        event: KeyEvent,
        _client: &mut Client<'_>,
        _app: &mut State,
        _shared: &mut SharedUiState,
    ) -> Result<KeyHandleResultInternal>;
}

#[derive(Debug, Display, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub enum CommonAction {
    Down,
    Up,
    MoveDown,
    MoveUp,
    DownHalf,
    UpHalf,
    Right,
    Left,
    Top,
    Bottom,
    EnterSearch,
    NextResult,
    PreviousResult,
    Select,
    Add,
    Delete,
    Rename,
    Close,
    Confirm,
    FocusInput,
}

impl Screens {
    pub fn next(self) -> Self {
        match self {
            #[cfg(debug_assertions)]
            Screens::Queue => Screens::Logs,
            #[cfg(not(debug_assertions))]
            Screens::Queue => Screens::Directories,
            #[cfg(debug_assertions)]
            Screens::Logs => Screens::Directories,
            Screens::Directories => Screens::Artists,
            Screens::Artists => Screens::Albums,
            Screens::Albums => Screens::Playlists,
            Screens::Playlists => Screens::Queue,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Screens::Queue => Screens::Playlists,
            Screens::Playlists => Screens::Albums,
            Screens::Albums => Screens::Artists,
            Screens::Artists => Screens::Directories,
            #[cfg(not(debug_assertions))]
            Screens::Directories => Screens::Queue,
            #[cfg(debug_assertions)]
            Screens::Directories => Screens::Logs,
            #[cfg(debug_assertions)]
            Screens::Logs => Screens::Queue,
        }
    }
}

pub mod dirstack {}

pub(crate) mod browser {
    use std::cmp::Ordering;

    use ratatui::{
        style::{Color, Style},
        text::{Line, Span},
        widgets::ListItem,
    };

    use crate::{
        config::SymbolsConfig,
        mpd::commands::{lsinfo::FileOrDir, Song},
    };

    impl Song {
        pub(crate) fn to_preview(&self, _symbols: &SymbolsConfig) -> impl Iterator<Item = ListItem<'static>> {
            let key_style = Style::default().fg(Color::Yellow);
            let separator = Span::from(": ");
            let start_of_line_spacer = Span::from(" ");

            let title = Line::from(vec![
                start_of_line_spacer.clone(),
                Span::styled("Title", key_style),
                separator.clone(),
                Span::from(self.title.as_ref().map_or("Untitled", |v| v.as_str()).to_owned()),
            ]);
            let artist = Line::from(vec![
                start_of_line_spacer.clone(),
                Span::styled("Artist", key_style),
                separator.clone(),
                Span::from(self.artist.as_ref().map_or("Unknown", |v| v.as_str()).to_owned()),
            ]);
            let album = Line::from(vec![
                start_of_line_spacer.clone(),
                Span::styled("Album", key_style),
                separator.clone(),
                Span::from(self.album.as_ref().map_or("Unknown", |v| v.as_str()).to_owned()),
            ]);
            let duration = Line::from(vec![
                start_of_line_spacer.clone(),
                Span::styled("Duration", key_style),
                separator.clone(),
                Span::from(
                    self.duration
                        .as_ref()
                        .map_or("-".to_owned(), |v| v.as_secs().to_string()),
                ),
            ]);
            let mut r = vec![title, artist, album, duration];
            for (k, v) in &self.others {
                r.push(Line::from(vec![
                    start_of_line_spacer.clone(),
                    Span::styled(k.clone(), key_style),
                    separator.clone(),
                    Span::from(v.clone()),
                ]));
            }

            r.into_iter().map(ListItem::new)
        }
    }
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) enum DirOrSong {
        Dir(String),
        Song(String),
    }

    impl DirOrSong {
        pub fn value(&self) -> &str {
            match self {
                DirOrSong::Dir(v) => v,
                DirOrSong::Song(v) => v,
            }
        }
    }

    impl std::cmp::Ord for DirOrSong {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            match (self, other) {
                (DirOrSong::Dir(a), DirOrSong::Dir(b)) => a.cmp(b),
                (_, DirOrSong::Dir(_)) => Ordering::Greater,
                (DirOrSong::Dir(_), _) => Ordering::Less,
                (DirOrSong::Song(a), DirOrSong::Song(b)) => a.cmp(b),
            }
        }
    }
    impl std::cmp::PartialOrd for DirOrSong {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl From<FileOrDir> for DirOrSong {
        fn from(value: FileOrDir) -> Self {
            match value {
                FileOrDir::Dir(dir) => DirOrSong::Dir(dir.path),
                FileOrDir::File(song) => DirOrSong::Song(song.file),
            }
        }
    }
}

pub trait SongExt {
    fn title_str(&self) -> &str;
    fn artist_str(&self) -> &str;
}

impl SongExt for Song {
    fn title_str(&self) -> &str {
        self.title.as_ref().map_or("Untitled", |v| v.as_str())
    }

    fn artist_str(&self) -> &str {
        self.artist.as_ref().map_or("Untitled", |v| v.as_str())
    }
}

pub(crate) trait StringExt {
    fn file_name(&self) -> &str;
    fn ellipsize(&self, max_len: usize) -> Cow<str>;
}

impl StringExt for String {
    fn file_name(&self) -> &str {
        self.rsplit('/')
            .next()
            .map_or(self, |v| v.rsplit_once('.').map_or(v, |v| v.0))
    }

    fn ellipsize(&self, max_len: usize) -> Cow<str> {
        if self.chars().count() > max_len {
            Cow::Owned(format!("{}...", self.chars().take(max_len - 3).collect::<String>()))
        } else {
            Cow::Borrowed(self)
        }
    }
}

enum MoveDirection {
    Up,
    Down,
}

#[allow(unused)]
trait BrowserScreen<T: DirStackItem + std::fmt::Debug>: Screen {
    fn stack(&self) -> &DirStack<T>;
    fn stack_mut(&mut self) -> &mut DirStack<T>;
    fn set_filter_input_mode_active(&mut self, active: bool);
    fn is_filter_input_mode_active(&self) -> bool;
    fn next(&mut self, client: &mut Client<'_>, shared: &mut SharedUiState) -> Result<KeyHandleResultInternal>;
    fn move_selected(
        &mut self,
        direction: MoveDirection,
        client: &mut Client<'_>,
        shared: &mut SharedUiState,
    ) -> Result<KeyHandleResultInternal> {
        Ok(KeyHandleResultInternal::SkipRender)
    }
    fn prepare_preview(&mut self, client: &mut Client<'_>, state: &State) -> Result<Option<Vec<ListItem<'static>>>>;
    fn add(&self, item: &T, client: &mut Client<'_>, shared: &mut SharedUiState) -> Result<KeyHandleResultInternal>;
    fn delete(
        &self,
        item: &T,
        index: usize,
        client: &mut Client<'_>,
        shared: &mut SharedUiState,
    ) -> Result<KeyHandleResultInternal> {
        Ok(KeyHandleResultInternal::SkipRender)
    }
    fn rename(&self, item: &T, client: &mut Client<'_>, shared: &mut SharedUiState) -> Result<KeyHandleResultInternal> {
        Ok(KeyHandleResultInternal::SkipRender)
    }
    fn handle_filter_input(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Char(c) => {
                if let Some(ref mut f) = self.stack_mut().current_mut().filter {
                    f.push(c);
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut f) = self.stack_mut().current_mut().filter {
                    f.pop();
                };
            }
            KeyCode::Enter => {
                self.set_filter_input_mode_active(false);
                self.stack_mut().current_mut().jump_next_matching();
            }
            KeyCode::Esc => {
                self.set_filter_input_mode_active(false);
                self.stack_mut().current_mut().filter = None;
            }
            _ => {}
        }
    }

    fn handle_common_action(
        &mut self,
        action: CommonAction,
        client: &mut Client<'_>,
        app: &mut State,
        shared: &mut SharedUiState,
    ) -> Result<KeyHandleResultInternal> {
        match action {
            CommonAction::Up => {
                self.stack_mut().current_mut().prev();
                let preview = self.prepare_preview(client, app).context("Cannot prepare preview")?;
                self.stack_mut().set_preview(preview);
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::Down => {
                self.stack_mut().current_mut().next();
                let preview = self.prepare_preview(client, app).context("Cannot prepare preview")?;
                self.stack_mut().set_preview(preview);
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::MoveUp => {
                let res = self.move_selected(MoveDirection::Up, client, shared)?;
                self.refresh(client, app, shared)?;
                Ok(res)
            }
            CommonAction::MoveDown => {
                let res = self.move_selected(MoveDirection::Down, client, shared)?;
                self.refresh(client, app, shared)?;
                Ok(res)
            }
            CommonAction::DownHalf => {
                self.stack_mut().current_mut().next_half_viewport();
                let preview = self.prepare_preview(client, app).context("Cannot prepare preview")?;
                self.stack_mut().set_preview(preview);
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::UpHalf => {
                self.stack_mut().current_mut().prev_half_viewport();
                let preview = self.prepare_preview(client, app).context("Cannot prepare preview")?;
                self.stack_mut().set_preview(preview);
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::Bottom => {
                self.stack_mut().current_mut().last();
                let preview = self.prepare_preview(client, app).context("Cannot prepare preview")?;
                self.stack_mut().set_preview(preview);
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::Top => {
                self.stack_mut().current_mut().first();
                let preview = self.prepare_preview(client, app).context("Cannot prepare preview")?;
                self.stack_mut().set_preview(preview);
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::Right => {
                let res = self.next(client, shared)?;
                let preview = self.prepare_preview(client, app).context("Cannot prepare preview")?;
                self.stack_mut().set_preview(preview);
                Ok(res)
            }
            CommonAction::Left => {
                self.stack_mut().pop();
                let preview = self.prepare_preview(client, app).context("Cannot prepare preview")?;
                self.stack_mut().set_preview(preview);
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::EnterSearch => {
                self.set_filter_input_mode_active(true);
                self.stack_mut().current_mut().filter = Some(String::new());
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::NextResult => {
                self.stack_mut().current_mut().jump_next_matching();
                let preview = self.prepare_preview(client, app).context("Cannot prepare preview")?;
                self.stack_mut().set_preview(preview);
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::PreviousResult => {
                self.stack_mut().current_mut().jump_previous_matching();
                let preview = self.prepare_preview(client, app).context("Cannot prepare preview")?;
                self.stack_mut().set_preview(preview);
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::Select => {
                self.stack_mut().current_mut().toggle_mark_selected();
                self.stack_mut().current_mut().next();
                let preview = self.prepare_preview(client, app).context("Cannot prepare preview")?;
                self.stack_mut().set_preview(preview);
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::Add if !self.stack().current().marked().is_empty() => {
                for idx in self.stack().current().marked().iter().rev() {
                    let item = &self.stack().current().items[*idx];
                    self.add(item, client, shared)?;
                }
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::Add => {
                if let Some(item) = self.stack().current().selected() {
                    self.add(item, client, shared)
                } else {
                    Ok(KeyHandleResultInternal::SkipRender)
                }
            }
            CommonAction::Delete if !self.stack().current().marked().is_empty() => {
                for idx in self.stack().current().marked().iter().rev() {
                    let item = &self.stack().current().items[*idx];
                    self.delete(item, *idx, client, shared)?;
                }
                self.refresh(client, app, shared)?;
                Ok(KeyHandleResultInternal::RenderRequested)
            }
            CommonAction::Delete => {
                if let Some((item, index)) = self.stack().current().selected_with_idx() {
                    self.delete(item, index, client, shared)?;
                    self.refresh(client, app, shared)?;
                    Ok(KeyHandleResultInternal::RenderRequested)
                } else {
                    Ok(KeyHandleResultInternal::SkipRender)
                }
            }
            CommonAction::Rename => {
                if let Some(item) = self.stack().current().selected() {
                    self.rename(item, client, shared)
                } else {
                    Ok(KeyHandleResultInternal::SkipRender)
                }
            }
            CommonAction::FocusInput => Ok(KeyHandleResultInternal::SkipRender),
            CommonAction::Close => Ok(KeyHandleResultInternal::SkipRender), // todo out?
            CommonAction::Confirm => Ok(KeyHandleResultInternal::SkipRender), // todo next?
        }
    }
}
