use crate::install::Stage;

#[derive(Debug)]
pub enum MessageType {
    Info,
}

#[derive(Debug)]
pub enum Event {
    EnterStage(Stage),
    ExitStage(Stage),
    Message(MessageType, String),
}
