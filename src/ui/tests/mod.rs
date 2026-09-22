//! The drawing tests, one file per drawing module.
//!
//! The items below are what the tests reach for as `super::…`, so a test moved here from
//! `src/ui.rs` keeps the paths it was written with.

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::code::{digits, with_find};
use super::draw;
use super::welcome::{LOGO, WELCOME_ACTIONS, welcome_hints};

mod code;
mod overlays;
mod welcome;

fn rows(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buf = terminal.backend().buffer();
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim()
                .to_string()
        })
        .collect()
}
