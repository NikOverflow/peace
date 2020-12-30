use config::Config;

use super::model::Logger;
use log::LevelFilter;

impl Logger {
    /// Init logger
    pub fn init(cfg: &Config) {
        let level = cfg.get_str("logger.level").unwrap_or("info".to_string());
        let env = env_logger::Env::default()
            // Try to get LOG_FILTER from .env,
            // If not exists, try use cfg
            .filter_or(
                "LOG_FILTER",
                cfg.get_str(&format!("logger.mode.{}", &level))
                    .unwrap_or("info".to_string()),
            );

        //env_logger::init_from_env(env);
        env_logger::Builder::from_env(env)
            // Disabled maxminddb debug log (so many wtf)
            .filter_module("maxminddb::decoder", LevelFilter::Warn)
            .format_timestamp_millis()
            .init();
    }
}
