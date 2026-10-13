use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::code::{digits, with_find};
use super::draw;
use super::welcome::{LOGO, WELCOME_ACTIONS, welcome_hints};

mod code;
mod overlays;
mod picture;
mod preview;
mod snapshots;
mod status;
mod welcome;

fn rows(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buf = terminal.backend().buffer();
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim()
                .to_owned()
        })
        .collect()
}
