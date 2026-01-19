pub mod error;
pub mod events;
pub mod music;

pub use error::on_error;
pub use events::handle_event;
pub use music::handle_track_end;
