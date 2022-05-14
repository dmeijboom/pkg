mod event;
mod installer;

pub use event::{Event, MessageType};
pub use installer::{Installer, Opts, Stage};

pub mod channel {
    use super::Event;
    use tokio::sync::mpsc;

    pub type Sender = mpsc::Sender<Event>;
    pub type Receiver = mpsc::Receiver<Event>;
}
