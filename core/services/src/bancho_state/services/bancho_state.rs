use super::{BanchoStateService, UserSessions};
use crate::bancho_state::{
    BanchoStateError, CreateSessionError, DynBanchoStateBackgroundService,
    DynBanchoStateService, DynUserSessionsService, GameMode, Mods,
    PresenceFilter, Session, UserOnlineStatus,
};
use async_trait::async_trait;
use bancho_packets::PacketBuilder;
use num_traits::FromPrimitive;
use peace_domain::bancho_state::CreateSessionDto;
use peace_pb::{
    bancho_state::{bancho_state_rpc_client::BanchoStateRpcClient, *},
    base::ExecSuccess,
};
use std::{collections::hash_map::Values, sync::Arc};
use tonic::{transport::Channel, Code};
use tools::atomic::AtomicValue;

#[derive(Clone)]
pub enum BanchoStateServiceImpl {
    Remote(BanchoStateServiceRemote),
    Local(BanchoStateServiceLocal),
}

impl BanchoStateServiceImpl {
    pub fn into_service(self) -> DynBanchoStateService {
        Arc::new(self) as DynBanchoStateService
    }

    pub fn remote(client: BanchoStateRpcClient<Channel>) -> Self {
        Self::Remote(BanchoStateServiceRemote(client))
    }

    pub fn local(
        user_sessions_service: DynUserSessionsService,
        bancho_state_background_service: DynBanchoStateBackgroundService,
    ) -> Self {
        Self::Local(BanchoStateServiceLocal::new(
            user_sessions_service,
            bancho_state_background_service,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct BanchoStateServiceRemote(BanchoStateRpcClient<Channel>);

impl BanchoStateServiceRemote {
    pub fn new(client: BanchoStateRpcClient<Channel>) -> Self {
        Self(client)
    }

    pub fn client(&self) -> BanchoStateRpcClient<Channel> {
        self.0.clone()
    }
}

#[derive(Clone)]
pub struct BanchoStateServiceLocal {
    user_sessions_service: DynUserSessionsService,
    #[allow(dead_code)]
    bancho_state_background_service: DynBanchoStateBackgroundService,
}

impl BanchoStateServiceLocal {
    pub fn new(
        user_sessions_service: DynUserSessionsService,
        bancho_state_background_service: DynBanchoStateBackgroundService,
    ) -> Self {
        Self { user_sessions_service, bancho_state_background_service }
    }
}

pub struct SessionFilter;

impl SessionFilter {
    pub fn session_is_target(
        session: &Session,
        to: &BanchoPacketTarget,
    ) -> bool {
        match to {
            BanchoPacketTarget::SessionId(t) if &session.id == t => true,
            BanchoPacketTarget::UserId(t) if &session.user_id == t => true,
            BanchoPacketTarget::Username(t)
                if session.username.load().as_ref() == t =>
            {
                true
            },
            BanchoPacketTarget::UsernameUnicode(t) => {
                if let Some(n) = session.username_unicode.load().as_deref() {
                    n == t
                } else {
                    false
                }
            },
            _ => false,
        }
    }
}

#[async_trait]
impl BanchoStateService for BanchoStateServiceImpl {
    async fn broadcast_bancho_packets(
        &self,
        request: BroadcastBanchoPacketsRequest,
    ) -> Result<ExecSuccess, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .broadcast_bancho_packets(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                let packet = Arc::new(request.packets);

                let user_sessions =
                    svc.user_sessions_service.user_sessions().read().await;

                for session in user_sessions.values() {
                    session.push_packet(packet.clone()).await;
                }

                Ok(ExecSuccess {})
            },
        }
    }

    async fn enqueue_bancho_packets(
        &self,
        request: EnqueueBanchoPacketsRequest,
    ) -> Result<ExecSuccess, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .enqueue_bancho_packets(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                let EnqueueBanchoPacketsRequest { target, packets } = request;

                let packet = Arc::new(packets);
                let target = Into::<BanchoPacketTarget>::into(
                    target.ok_or(BanchoStateError::InvalidArgument)?,
                );

                if let Ok(user_query) = TryInto::<UserQuery>::try_into(target) {
                    svc.user_sessions_service
                        .get(&user_query)
                        .await
                        .ok_or(BanchoStateError::SessionNotExists)?
                        .push_packet(packet)
                        .await;
                } else {
                    todo!("channel handle")
                }

                Ok(ExecSuccess {})
            },
        }
    }

    async fn batch_enqueue_bancho_packets(
        &self,
        request: BatchEnqueueBanchoPacketsRequest,
    ) -> Result<ExecSuccess, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .batch_enqueue_bancho_packets(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                let BatchEnqueueBanchoPacketsRequest { targets, packets } =
                    request;
                let packets = Arc::new(packets);

                let user_sessions =
                    svc.user_sessions_service.user_sessions().read().await;

                for target in targets {
                    let target = Into::<BanchoPacketTarget>::into(target);

                    if let Ok(user_query) =
                        TryInto::<UserQuery>::try_into(target)
                    {
                        if let Some(session) =
                            UserSessions::get_inner(&user_sessions, &user_query)
                        {
                            session.push_packet(packets.clone()).await;
                        }
                    } else {
                        todo!("channel handle")
                    }
                }

                Ok(ExecSuccess {})
            },
        }
    }

    async fn dequeue_bancho_packets(
        &self,
        request: DequeueBanchoPacketsRequest,
    ) -> Result<BanchoPackets, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .dequeue_bancho_packets(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                let target = Into::<BanchoPacketTarget>::into(
                    request.target.ok_or(BanchoStateError::InvalidArgument)?,
                );

                let mut data = Vec::new();

                if let Ok(user_query) = TryInto::<UserQuery>::try_into(target) {
                    let session = svc
                        .user_sessions_service
                        .get(&user_query)
                        .await
                        .ok_or(BanchoStateError::SessionNotExists)?;

                    let mut session_packet_queue =
                        session.packets_queue.lock().await;

                    while let Some(packet) = session
                        .dequeue_packet(Some(&mut session_packet_queue))
                        .await
                    {
                        data.extend(packet.iter());
                    }
                } else {
                    todo!("channel handle")
                }

                Ok(BanchoPackets { data })
            },
        }
    }

    async fn create_user_session(
        &self,
        request: CreateUserSessionRequest,
    ) -> Result<CreateUserSessionResponse, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .create_user_session(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                let CreateUserSessionRequest {
                    user_id,
                    username,
                    username_unicode,
                    privileges,
                    client_version,
                    utc_offset,
                    display_city,
                    only_friend_pm_allowed,
                    connection_info,
                } = request;

                // Create a new user session using the provided request.
                let session = svc
                    .user_sessions_service
                    .create(CreateSessionDto {
                        user_id,
                        username,
                        username_unicode,
                        privileges,
                        client_version,
                        utc_offset: utc_offset as u8,
                        display_city,
                        only_friend_pm_allowed,
                        connection_info: connection_info
                            .ok_or(CreateSessionError::InvalidConnectionInfo)?
                            .into(),
                        initial_packets: None,
                    })
                    .await;

                // Return the new session ID in a response.
                Ok(CreateUserSessionResponse {
                    session_id: session.id.to_owned(),
                })
            },
        }
    }

    async fn delete_user_session(
        &self,
        query: UserQuery,
    ) -> Result<ExecSuccess, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .delete_user_session(Into::<RawUserQuery>::into(query))
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                svc.user_sessions_service.delete(&query).await;
                Ok(ExecSuccess {})
            },
        }
    }

    async fn check_user_session_exists(
        &self,
        query: UserQuery,
    ) -> Result<UserSessionExistsResponse, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .check_user_session_exists(Into::<RawUserQuery>::into(query))
                .await
                .map_err(|status| {
                    if status.code() == Code::NotFound {
                        BanchoStateError::SessionNotExists
                    } else {
                        BanchoStateError::RpcError(status)
                    }
                })
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                // Get session based on the provided query
                let user_id = {
                    let session = svc
                        .user_sessions_service
                        .get(&query)
                        .await
                        .ok_or(BanchoStateError::SessionNotExists)?;

                    session.update_active();
                    session.user_id
                };

                // Return the user ID in a response.
                Ok(UserSessionExistsResponse { user_id })
            },
        }
    }

    async fn get_user_session(
        &self,
        query: UserQuery,
    ) -> Result<GetUserSessionResponse, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .get_user_session(Into::<RawUserQuery>::into(query))
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                // Get session based on the provided query
                let session = svc
                    .user_sessions_service
                    .get(&query)
                    .await
                    .ok_or(BanchoStateError::SessionNotExists)?;

                // Create a response with the user session details
                Ok(GetUserSessionResponse {
                    // Copy the session ID into the response
                    session_id: Some(session.id.to_owned()),
                    // Copy the user ID into the response
                    user_id: Some(session.user_id),
                    // Copy the username into the response
                    username: Some(session.username.to_string()),
                    // Copy the Unicode username into the response, if it exists
                    username_unicode: session
                        .username_unicode
                        .load()
                        .as_ref()
                        .map(|s| s.to_string()),
                })
            },
        }
    }

    async fn get_user_session_with_fields(
        &self,
        request: RawUserQueryWithFields,
    ) -> Result<GetUserSessionResponse, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .get_user_session_with_fields(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                // Extract the query and fields from the request
                let query = request
                    .user_query
                    .ok_or(BanchoStateError::InvalidArgument)?;

                // Get session based on the provided query
                let session = svc
                    .user_sessions_service
                    .get(&query.into())
                    .await
                    .ok_or(BanchoStateError::SessionNotExists)?;

                // Initialize the response and extract the requested fields
                let mut res = GetUserSessionResponse::default();
                let fields = UserSessionFields::from(request.fields);

                // Set the response fields based on the requested fields
                if fields.intersects(UserSessionFields::SessionId) {
                    res.session_id = Some(session.id.to_owned());
                }

                if fields.intersects(UserSessionFields::UserId) {
                    res.user_id = Some(session.user_id);
                }

                if fields.intersects(UserSessionFields::Username) {
                    res.username = Some(session.username.to_string());
                }

                if fields.intersects(UserSessionFields::UsernameUnicode) {
                    res.username_unicode = session
                        .username_unicode
                        .load()
                        .as_ref()
                        .map(|s| s.to_string());
                }

                // Return the response
                Ok(res)
            },
        }
    }

    async fn channel_update_notify(
        &self,
        request: ChannelUpdateNotifyRequest,
    ) -> Result<ChannelUpdateNotifyResponse, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .channel_update_notify(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                let ChannelUpdateNotifyRequest { notify_targets, channel_info } =
                    request;

                let channel_info =
                    channel_info.ok_or(BanchoStateError::InvalidArgument)?;

                let packets = Arc::new(
                    PacketBuilder::new()
                        .add(bancho_packets::ChannelInfo::pack(
                            channel_info.name.as_str().into(),
                            channel_info
                                .description
                                .map(|s| s.into())
                                .unwrap_or_default(),
                            channel_info
                                .counter
                                .ok_or(BanchoStateError::InvalidArgument)?
                                .bancho as i16,
                        ))
                        .build(),
                );

                let fails = {
                    let mut fails = Vec::<RawUserQuery>::new();
                    let indexes =
                        svc.user_sessions_service.user_sessions().read().await;

                    match notify_targets {
                        Some(notify_targets) => {
                            let notify_targets = notify_targets
                                .value
                                .into_iter()
                                .map(|t| t.into())
                                .collect::<Vec<UserQuery>>();

                            for user_query in notify_targets {
                                if let Some(session) = UserSessions::get_inner(
                                    &indexes,
                                    &user_query,
                                ) {
                                    session.push_packet(packets.clone()).await;
                                } else {
                                    fails.push(user_query.into())
                                }
                            }
                        },
                        None => {
                            for session in indexes.values() {
                                session.push_packet(packets.clone()).await;
                            }
                        },
                    }

                    fails
                };

                Ok(ChannelUpdateNotifyResponse { fails })
            },
        }
    }

    async fn get_all_sessions(
        &self,
    ) -> Result<GetAllSessionsResponse, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .get_all_sessions(GetAllSessionsRequest {})
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                // Get a read lock on the `user_sessions` hash map
                let user_sessions = svc.user_sessions_service.user_sessions();
                let indexes = user_sessions.read().await;

                #[inline]
                fn collect_data<K>(
                    values: Values<'_, K, Arc<Session>>,
                ) -> Vec<UserData> {
                    values
                        .map(|session| UserData {
                            json: serde_json::to_string(session)
                                .unwrap_or_else(|err| {
                                    format!("err: {:?}", err)
                                }),
                        })
                        .collect()
                }

                // Collect session data by index
                let indexed_by_session_id =
                    collect_data(indexes.session_id.values());
                let indexed_by_user_id = collect_data(indexes.user_id.values());
                let indexed_by_username =
                    collect_data(indexes.username.values());
                let indexed_by_username_unicode =
                    collect_data(indexes.username_unicode.values());

                // Return a `GetAllSessionsResponse` message containing the
                // session data
                Ok(GetAllSessionsResponse {
                    len: user_sessions.len() as u64,
                    indexed_by_session_id,
                    indexed_by_user_id,
                    indexed_by_username,
                    indexed_by_username_unicode,
                })
            },
        }
    }

    async fn send_user_stats_packet(
        &self,
        request: SendUserStatsPacketRequest,
    ) -> Result<ExecSuccess, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .send_user_stats_packet(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                let to = request.to.ok_or(BanchoStateError::InvalidArgument)?;

                // Extract the query and fields from the request
                let query = request
                    .user_query
                    .ok_or(BanchoStateError::InvalidArgument)?;

                // Get session based on the provided query
                let session = svc
                    .user_sessions_service
                    .get(&query.into())
                    .await
                    .ok_or(BanchoStateError::SessionNotExists)?;

                self.enqueue_bancho_packets(EnqueueBanchoPacketsRequest {
                    target: Some(to),
                    packets: session.user_stats_packet(),
                })
                .await?;

                Ok(ExecSuccess {})
            },
        }
    }

    async fn batch_send_user_stats_packet(
        &self,
        request: BatchSendUserStatsPacketRequest,
    ) -> Result<ExecSuccess, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .batch_send_user_stats_packet(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                if request.user_queries.is_empty() {
                    return Ok(ExecSuccess {});
                }
                let to =
                    request.to.ok_or(BanchoStateError::InvalidArgument)?.into();

                let user_stats_packets = {
                    let mut user_stats_packets = Vec::new();

                    let indexes =
                        svc.user_sessions_service.user_sessions().read().await;

                    for raw_query in request.user_queries {
                        let query = raw_query.into();
                        let session = match &query {
                            UserQuery::UserId(user_id) => {
                                indexes.user_id.get(user_id)
                            },
                            UserQuery::Username(username) => {
                                indexes.username.get(username)
                            },
                            UserQuery::UsernameUnicode(username_unicode) => {
                                indexes.username_unicode.get(username_unicode)
                            },
                            UserQuery::SessionId(session_id) => {
                                indexes.session_id.get(session_id)
                            },
                        };

                        let session = match session {
                            Some(s) => s,
                            None => continue,
                        };

                        if SessionFilter::session_is_target(session, &to) {
                            continue;
                        };

                        user_stats_packets.extend(session.user_stats_packet());
                    }

                    user_stats_packets
                };

                self.enqueue_bancho_packets(EnqueueBanchoPacketsRequest {
                    target: Some(to.into()),
                    packets: user_stats_packets,
                })
                .await?;

                Ok(ExecSuccess {})
            },
        }
    }

    async fn send_all_presences(
        &self,
        request: SendAllPresencesRequest,
    ) -> Result<ExecSuccess, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .send_all_presences(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                let to = Into::<BanchoPacketTarget>::into(
                    request.to.ok_or(BanchoStateError::InvalidArgument)?,
                );

                let presences_packets = {
                    let mut presences_packets = Vec::new();

                    let user_sessions =
                        svc.user_sessions_service.user_sessions().read().await;

                    for session in user_sessions.values() {
                        if SessionFilter::session_is_target(session, &to) {
                            continue;
                        };

                        presences_packets
                            .extend(session.user_presence_packet());
                    }

                    presences_packets
                };

                self.enqueue_bancho_packets(EnqueueBanchoPacketsRequest {
                    target: Some(to.into()),
                    packets: presences_packets,
                })
                .await?;

                Ok(ExecSuccess {})
            },
        }
    }

    async fn batch_send_presences(
        &self,
        request: BatchSendPresencesRequest,
    ) -> Result<ExecSuccess, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .batch_send_presences(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                if request.user_queries.is_empty() {
                    return Ok(ExecSuccess {});
                }
                let to = Into::<BanchoPacketTarget>::into(
                    request.to.ok_or(BanchoStateError::InvalidArgument)?,
                );

                let presences_packets = {
                    let mut presences_packets = Vec::new();

                    let indexes =
                        svc.user_sessions_service.user_sessions().read().await;

                    for raw_query in request.user_queries {
                        let query = raw_query.into();
                        let session = match &query {
                            UserQuery::UserId(user_id) => {
                                indexes.user_id.get(user_id)
                            },
                            UserQuery::Username(username) => {
                                indexes.username.get(username)
                            },
                            UserQuery::UsernameUnicode(username_unicode) => {
                                indexes.username_unicode.get(username_unicode)
                            },
                            UserQuery::SessionId(session_id) => {
                                indexes.session_id.get(session_id)
                            },
                        };

                        let session = match session {
                            Some(s) => s,
                            None => continue,
                        };

                        if SessionFilter::session_is_target(session, &to) {
                            continue;
                        };

                        presences_packets
                            .extend(session.user_presence_packet());
                    }

                    presences_packets
                };

                self.enqueue_bancho_packets(EnqueueBanchoPacketsRequest {
                    target: Some(to.into()),
                    packets: presences_packets,
                })
                .await?;

                Ok(ExecSuccess {})
            },
        }
    }

    async fn update_presence_filter(
        &self,
        request: UpdatePresenceFilterRequest,
    ) -> Result<ExecSuccess, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .update_presence_filter(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                // Extract the query and fields from the request
                let query = request
                    .user_query
                    .ok_or(BanchoStateError::InvalidArgument)?;

                let presence_filter =
                    PresenceFilter::from_i32(request.presence_filter)
                        .ok_or(BanchoStateError::InvalidArgument)?;

                // Get session based on the provided query
                let session = svc
                    .user_sessions_service
                    .get(&query.into())
                    .await
                    .ok_or(BanchoStateError::SessionNotExists)?;

                session.presence_filter.set(presence_filter.into());

                Ok(ExecSuccess {})
            },
        }
    }

    async fn update_user_bancho_status(
        &self,
        request: UpdateUserBanchoStatusRequest,
    ) -> Result<ExecSuccess, BanchoStateError> {
        match self {
            Self::Remote(svc) => svc
                .client()
                .update_user_bancho_status(request)
                .await
                .map_err(BanchoStateError::RpcError)
                .map(|resp| resp.into_inner()),
            Self::Local(svc) => {
                let UpdateUserBanchoStatusRequest {
                    user_query,
                    online_status,
                    description,
                    beatmap_md5,
                    mods,
                    mode,
                    beatmap_id,
                } = request;

                // Extract the query and fields from the request
                let query =
                    user_query.ok_or(BanchoStateError::InvalidArgument)?;

                let session = svc
                    .user_sessions_service
                    .get(&query.into())
                    .await
                    .ok_or(BanchoStateError::SessionNotExists)?;

                let online_status = UserOnlineStatus::from_i32(online_status)
                    .unwrap_or_default();
                let mods = Mods::from(mods);
                let mode = GameMode::from_i32(mode).unwrap_or_default();

                session.bancho_status.update_all(
                    online_status,
                    description,
                    beatmap_id,
                    beatmap_md5,
                    mods,
                    mode,
                );

                // todo update stats from database

                self.broadcast_bancho_packets(BroadcastBanchoPacketsRequest {
                    packets: session.user_stats_packet(),
                })
                .await?;

                Ok(ExecSuccess {})
            },
        }
    }
}
