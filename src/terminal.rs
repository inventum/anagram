//! Terminal raw mode and polling.

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::style::{Attribute, SetAttribute};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::io::{self, stdout, Write};

pub struct TerminalGuard;

impl TerminalGuard {
    pub fn enter() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, SetAttribute(Attribute::Reset), EnterAlternateScreen, Hide)?;
        out.flush()?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut out = stdout();
        let _ = execute!(out, Show, LeaveAlternateScreen);
        let _ = out.flush();
    }
}

pub fn poll_event() -> io::Result<Option<Event>> {
    if event::poll(std::time::Duration::from_millis(100))? {
        event::read().map(Some)
    } else {
        Ok(None)
    }
}
