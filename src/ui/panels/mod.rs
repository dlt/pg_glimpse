mod blocking;
mod extensions;
mod indexes;
mod replication;
mod settings;
mod statements;
mod tables;
mod vacuum;
mod wait_events;
mod wal_io;
mod wraparound;

pub use blocking::render_blocking;
pub use extensions::render_extensions;
pub use indexes::render_indexes;
pub use replication::render_replication;
pub use settings::render_settings;
pub use statements::render_statements;
pub use tables::render_table_stats;
pub use vacuum::render_vacuum_progress;
pub use wait_events::render_wait_events;
pub use wal_io::render_wal_io;
pub use wraparound::render_wraparound;

use ratatui::widgets::{Block, BorderType, Borders};

use super::theme::Theme;

pub fn panel_block(title: &str) -> Block<'_> {
    Block::default()
        .title(format!(" {title} "))
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_style(Theme::border_active()))
}
