#![allow(dead_code)]
#![allow(unused_imports)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

extern crate config;
extern crate serde;

use async_std::sync::RwLock;
use colored::Colorize;

mod constants;
mod database;
mod handlers;
mod objects;
mod packets;
mod routes;
mod settings;
mod types;
mod utils;

use database::Database;
use objects::PlayerSessions;
use settings::{model::Settings, BANNER};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Print banner
    println!("{}", BANNER.green());

    // Create Peace's settings
    let (cfg, settings) = Settings::new().unwrap();

    // Create database object includes postgres and redis pool
    let database = Database::new(&settings).await;

    // Create PlayerSession for this server
    let player_sessions = RwLock::new(PlayerSessions::new(100, database.clone()));

    // Start actix server
    settings::actix::start_server(cfg, database, player_sessions).await
}
