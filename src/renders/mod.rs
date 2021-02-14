use std::sync::Arc;

use askama::Template;
use async_std::sync::RwLock;

use crate::settings::bancho::BanchoConfig;

#[derive(Template, Clone)]
#[template(path = "bancho_get.html")]
pub struct BanchoGet {
    pub server_name: String,
    pub server_front: String,
    bancho_config: Arc<RwLock<BanchoConfig>>,
}

impl BanchoGet {
    pub async fn new(bancho_config: Arc<RwLock<BanchoConfig>>) -> Self {
        let (server_name, server_front) = {
            let bc = bancho_config.read().await;
            (bc.server_name.clone(), bc.server_front_url.clone())
        };
        BanchoGet {
            server_name: server_name,
            server_front: server_front,
            bancho_config,
        }
    }

    #[inline(always)]
    pub async fn update(&mut self) {
        let (server_name, server_front) = {
            let bc = self.bancho_config.read().await;
            (bc.server_name.clone(), bc.server_front_url.clone())
        };
        self.server_name = server_name;
        self.server_front = server_front;
    }
}
