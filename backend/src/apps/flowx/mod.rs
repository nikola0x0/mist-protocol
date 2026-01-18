// FlowX DEX integration module
pub mod config;
pub mod routes;
// Old service module disabled - using new transaction-based approach
// pub mod services;
pub mod transaction;
pub mod utils;

pub use config::Config as FlowXConfig;
// pub use services::FlowXService;
