use std::fmt;

#[derive(Debug)]
pub enum BotError {
    Config(String),
    Client(String),
    Runtime(String),
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BotError::Config(msg) => write!(f, "Configuration error: {}", msg),
            BotError::Client(msg) => write!(f, "Client error: {}", msg),
            BotError::Runtime(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl std::error::Error for BotError {}

pub type Result<T> = std::result::Result<T, BotError>;