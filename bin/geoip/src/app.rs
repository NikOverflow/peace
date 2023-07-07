use crate::GeoipRpcImpl;
use clap_serde_derive::ClapSerde;
use peace_pb::geoip::{geoip_rpc_server::GeoipRpcServer, GEOIP_DESCRIPTOR_SET};
use peace_rpc::{RpcApplication, RpcFrameConfig};
use peace_runtime::cfg::RuntimeConfig;
use peace_services::{
    geoip::{FromGeoDbPath, GeoipServiceImpl},
    IntoService,
};
use std::sync::Arc;
use tonic::{
    async_trait,
    transport::{server::Router, Server},
};

/// PEACE Geo-ip gRPC service
#[peace_config]
#[command(name = "geoip", author, version, about, propagate_version = true)]
pub struct GeoipConfig {
    #[command(flatten)]
    pub runtime_cfg: RuntimeConfig,

    #[command(flatten)]
    pub frame_cfg: RpcFrameConfig,

    #[arg(long, short = 'P')]
    pub geo_db_path: Option<String>,
}

#[derive(Clone)]
pub struct App {
    pub cfg: Arc<GeoipConfig>,
}

impl App {
    pub fn new(cfg: Arc<GeoipConfig>) -> Self {
        Self { cfg }
    }
}

#[async_trait]
impl RpcApplication for App {
    fn frame_cfg(&self) -> &RpcFrameConfig {
        &self.cfg.frame_cfg
    }

    fn service_descriptors(&self) -> Option<&[&[u8]]> {
        Some(&[GEOIP_DESCRIPTOR_SET])
    }

    async fn service(&self, mut configured_server: Server) -> Router {
        let geo_db_path =
            self.cfg.geo_db_path.as_ref().expect("geo_db_path is required");

        let geoip_service = GeoipServiceImpl::from_path(geo_db_path.as_str())
            .unwrap()
            .into_service();

        let geoip_rpc = GeoipRpcImpl::new(geoip_service);

        configured_server.add_service(GeoipRpcServer::new(geoip_rpc))
    }
}
