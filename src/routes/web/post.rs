use super::depends::*;
use crate::handlers::web::post;
use crate::utils;

const BASE: &'static str = "Bancho /web [POST]";

pub async fn handler(
    req: HttpRequest,
    path: Path<String>,
    counter: Data<IntCounterVec>,
    player_sessions: Data<RwLock<PlayerSessions>>,
    database: Data<Database>,
    bancho_config: Data<RwLock<BanchoConfig>>,
    argon2_cache: Data<RwLock<Argon2Cache>>,
    geo_db: Data<Option<Reader<Mmap>>>,
    payload: Multipart,
) -> HttpResponse {
    counter.with_label_values(&["/web", "post", "start"]).inc();
    // Get real request ip
    let request_ip = match utils::get_realip(&req).await {
        Ok(ip) => ip,
        Err(_) => {
            return HttpResponse::BadRequest().body("bad requests");
        }
    };

    let ctx = || Context {
        req: &req,
        counter: &counter,
        player_sessions: &player_sessions,
        database: &database,
        bancho_config: &bancho_config,
        argon2_cache: &argon2_cache,
        geo_db: &geo_db,
    };

    debug!("{} Path: <{}>; ip: {}", BASE, path, request_ip);

    let handle_start = std::time::Instant::now();
    let handle_path = path.replace(".php", "");
    let resp = match handle_path.as_str() {
        /* "osu-session" => {} */
        "osu-error" => post::osu_error(&ctx(), payload).await,
        /* "osu-get-beatmapinfo" => {}
        "osu-submit-modular-selector" => {}
        "osu-comment" => {}
        "osu-screenshot" => {}
        "osu-osz2-bmsubmit-post" => {}
        "osu-osz2-bmsubmit-upload" => {} */
        _ => {
            warn!("{} Unimplemented path: <{}>", BASE, path);
            HttpResponse::Ok().body("ok")
        }
    };

    let handle_end = handle_start.elapsed();
    info!(
        "{} Path: <{}> done; time spent: {:?}",
        BASE, path, handle_end
    );

    resp
}
