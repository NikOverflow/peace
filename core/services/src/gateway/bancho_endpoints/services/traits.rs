use crate::gateway::bancho_endpoints::{
    extractors::{BanchoClientToken, BanchoClientVersion},
    BanchoHttpError, LoginError,
};
use async_trait::async_trait;
use axum::response::Response;
use bancho_packets::Packet;
use peace_pb::bancho_state::UserQuery;
use std::{net::IpAddr, sync::Arc};

pub type DynBanchoRoutingService = Arc<dyn BanchoRoutingService + Send + Sync>;
pub type DynBanchoHandlerService = Arc<dyn BanchoHandlerService + Send + Sync>;

#[async_trait]
pub trait BanchoRoutingService {
    /// get /
    async fn bancho_get(&self) -> Response;

    /// post /
    async fn bancho_post(
        &self,
        session_id: Option<BanchoClientToken>,
        version: Option<BanchoClientVersion>,
        ip: IpAddr,
        body: Vec<u8>,
    ) -> Result<Response, BanchoHttpError>;

    /// get /ss/{screenshot}
    async fn get_screenshot(&self) -> Response;

    /// get /d/{beatmapset_id}
    async fn download_beatmapset(&self, beatmapset_id: i32) -> Response;

    /// post /users
    async fn client_register(&self) -> Response;

    /// get /p/doyoureallywanttoaskpeppy
    async fn ask_peppy(&self) -> Response;

    /// get /difficulty-rating
    async fn difficulty_rating(&self) -> Response;

    /// post /web/osu-error.php
    async fn osu_error(&self) -> Response;

    /// post /web/osu-screenshot.php
    async fn osu_screenshot(&self) -> Response;

    /// get /web/osu-getfriends.php"
    async fn osu_getfriends(&self) -> Response;

    /// get /web/osu-getbeatmapinfo.php
    async fn osu_getbeatmapinfo(&self) -> Response;

    /// get /web/osu-getfavourites.php
    async fn osu_getfavourites(&self) -> Response;

    /// get /web/osu-addfavourite.php
    async fn osu_addfavourite(&self) -> Response;

    /// get /web/osu-lastfm.php
    async fn lastfm(&self) -> Response;

    /// get /web/osu-search.php
    async fn osu_search(&self) -> Response;

    /// get /web/osu-search-set.php
    async fn osu_search_set(&self) -> Response;

    /// post /web/osu-submit-modular-selector.php
    async fn osu_submit_modular_selector(&self) -> Response;

    /// get /web/osu-getreplay.php
    async fn osu_getreplay(&self) -> Response;

    /// get /web/osu-rate.php
    async fn osu_rate(&self) -> Response;

    /// get /web/osu-osz2-getscores.php
    async fn osu_osz2_getscores(&self) -> Response;

    /// post /web/osu-comment.php
    async fn osu_comment(&self) -> Response;

    /// get /web/osu-markasread.php
    async fn osu_markasread(&self) -> Response;

    /// get /web/osu-getseasonal.php
    async fn osu_getseasonal(&self) -> Response;

    /// get /web/bancho_connect.php
    async fn bancho_connect(&self) -> Response;

    /// get /web/check-updates.php
    async fn check_updates(&self) -> Response;

    /// get /web/maps/{beatmap_file_name}
    async fn update_beatmap(&self) -> Response;
}

#[async_trait]
pub trait BanchoHandlerService {
    async fn bancho_login(
        &self,
        body: Vec<u8>,
        client_ip: IpAddr,
        version: Option<BanchoClientVersion>,
    ) -> Result<Response, LoginError>;

    async fn bancho_post_responder(
        &self,
        user_id: i32,
        session_id: BanchoClientToken,
        body: Vec<u8>,
    ) -> Result<Response, BanchoHttpError>;

    async fn check_user_session(
        &self,
        query: UserQuery,
    ) -> Result<i32, BanchoHttpError>;

    async fn process_bancho_packet(
        &self,
        session_id: &str,
        _user_id: i32,
        packet: Packet<'_>,
    ) -> Result<(), BanchoHttpError>;
}
