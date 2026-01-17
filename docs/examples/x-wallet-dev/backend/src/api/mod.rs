//! API module - REST API handlers split by domain
//!
//! Structure:
//! - accounts: Account lookup, search, balance, NFTs
//! - transactions: Transaction history, NFT activities
//! - auth: Twitter OAuth, create account
//! - wallet: Link wallet
//! - sponsor: Enoki gas sponsorship
//! - tweets: Tweet status
//! - config: App configuration
//! - monitor: Enclave health monitoring

mod accounts;
mod auth;
mod config;
mod monitor;
mod sponsor;
mod transactions;
mod tweets;
mod types;
mod wallet;

// Re-export all handlers for use in main.rs routes
pub use accounts::*;
pub use auth::*;
pub use config::*;
pub use monitor::*;
pub use sponsor::*;
pub use transactions::*;
pub use tweets::*;
pub use wallet::*;
