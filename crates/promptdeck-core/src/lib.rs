pub mod clock;
pub mod error;
pub mod id;
pub mod markdown;
pub mod model;
pub mod paths;
pub mod search;
pub mod storage;
pub mod variables;

pub const APP_NAME: &str = "PromptDeck";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
