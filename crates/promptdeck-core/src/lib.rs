pub mod clock;
pub mod error;
pub mod id;
pub mod model;
pub mod paths;
pub mod storage;

pub const APP_NAME: &str = "PromptDeck";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
