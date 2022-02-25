use crate::common::ContentDisposition;

use {
    bytes::Bytes,
    futures::StreamExt,
    hashbrown::HashMap,
    ntex::http::header,
    ntex::web::{middleware::Logger, types::Data, HttpRequest},
    ntex_multipart::Multipart,
    std::str::FromStr,
    tokio::sync::RwLock,
    what_i_want::*,
};

#[derive(Debug)]
pub struct MultipartData {
    pub forms: HashMap<String, String>,
    pub files: HashMap<String, Vec<u8>>,
}

impl MultipartData {
    #[inline]
    pub fn form<T>(&mut self, key: &str) -> Option<T>
    where
        T: FromStr,
    {
        let s = self.forms.remove(key)?;
        T::from_str(s.as_ref()).ok()
    }

    #[inline]
    pub fn file(&mut self, key: &str) -> Option<Vec<u8>> {
        self.files.remove(key)
    }
}

#[inline]
/// Get deserialized multipart/form-data or files
pub async fn get_mutipart_data(mut mutipart_data: Multipart) -> MultipartData {
    let mut files = HashMap::new();
    let mut forms = HashMap::new();
    while let Some(Ok(mut field)) = mutipart_data.next().await {
        let disposition = unwrap_or_continue!(field.headers().get(&header::CONTENT_DISPOSITION));
        let disposition_str = unwrap_or_continue!(disposition.to_str());
        let dis = unwrap_or_continue!(ContentDisposition::parse(disposition_str));
        let key = dis.name.map(|s| s.to_string());
        let has_filename = dis.filename.is_some();
        let key = unwrap_or_continue!(key);
        while let Some(Ok(chunk)) = field.next().await {
            if has_filename {
                files.insert(key.to_string(), chunk.to_vec());
            } else {
                forms.insert(
                    key.to_string(),
                    String::from_utf8(chunk.to_vec()).unwrap_or(String::new()),
                );
            }
        }
    }
    MultipartData { forms, files }
}

#[inline]
/// Get deserialized multipart/form-data
///
/// use query method, some data types not support (such as bytes)
pub async fn simple_get_form_data<T: serde::de::DeserializeOwned>(
    mut form_data: Multipart,
) -> Result<T, serde_qs::Error> {
    let mut temp: String = String::new();
    while let Some(Ok(mut field)) = form_data.next().await {
        let disposition = unwrap_or_continue!(field.headers().get(&header::CONTENT_DISPOSITION));
        let disposition_str = unwrap_or_continue!(disposition.to_str());
        let key = unwrap_or_continue!(ContentDisposition::get_name(disposition_str)).to_string();
        while let Some(Ok(chunk)) = field.next().await {
            let value = String::from_utf8(chunk.to_vec()).unwrap_or(String::new());
            if temp.len() > 0 {
                temp.push('&');
            }
            temp.push_str(&format!("{}={}", key, value));
        }
    }
    serde_qs::from_str(&temp)
}

#[inline]
pub fn lock_wrapper<T>(obj: T) -> Data<RwLock<T>> {
    Data::new(RwLock::new(obj))
}

#[inline]
/// Get real ip from request
pub async fn get_realip(req: &HttpRequest) -> Result<String, ()> {
    Ok(req.connection_info().host().to_string())
}

#[inline]
pub fn header_checker(req: &HttpRequest, key: &str, value: &str) -> bool {
    let hv = unwrap_or_false!(req.headers().get(key));
    let v = unwrap_or_false!(hv.to_str());
    require!(v == value, false);
    true
}

#[inline]
/// Get osu version from headers
pub async fn get_osuver(req: &HttpRequest) -> String {
    unwrap_or_val!(req.headers().get("osu-version"), "unknown".to_string())
        .to_str()
        .unwrap_or("unknown")
        .to_string()
}

#[inline]
/// Get osu token from headers
pub async fn get_token(req: &HttpRequest) -> String {
    match req.headers().get("osu-token") {
        Some(version) => version.to_str().unwrap_or("unknown").to_string(),
        None => "unknown".to_string(),
    }
}

#[inline]
pub fn osu_sumit_token_checker(req: &HttpRequest) -> bool {
    if let Some(token) = req.headers().get("Token") {
        if let Ok(token) = token.to_str() {
            let token = token.split("|").collect::<Vec<&str>>();
            if token.len() == 2 && token[0].len() > 4000 && token[1].len() == 32 {
                return true;
            };
            warn!(
                "[osu_sumit_token_checker] Token len: {}, len[0]: {}, len[1]: {}",
                token.len(),
                token[0].len(),
                token[1].len()
            );
        };
    };
    false
}

pub fn make_logger(
    log_format: &str,
    exclude_target_endpoint: bool,
    target_endpoint: &str,
    exclude_endpoints: &Vec<String>,
    _exclude_endpoints_regex: &Vec<String>,
) -> Logger {
    let mut logger = match exclude_target_endpoint {
        true => Logger::new(log_format).exclude(target_endpoint),
        false => Logger::new(log_format),
    };
    for i in exclude_endpoints.iter() {
        logger = logger.exclude(i as &str);
    }
    // TODO: ntex is currently not supporting for regex
    /* for i in exclude_endpoints_regex.iter() {
        logger = logger.exclude_regex(i as &str);
    } */
    logger
}
