use super::{ChatService, DynChatService};
use crate::{chat::*, FromRpcClient, IntoService, RpcClient};
use async_trait::async_trait;
use bancho_packets::BanchoPacket;
use derive_deref::Deref;
use peace_pb::{
    bancho_state::{BanchoPackets, RawUserQuery, UserQuery},
    base::ExecSuccess,
    chat::{chat_rpc_client::ChatRpcClient, *},
};
use std::sync::Arc;
use tonic::transport::Channel as RpcChannel;
use tools::{
    atomic::U64,
    cache::{CachedAtomic, CachedValue},
    message_queue::ReceivedMessages,
};

pub const DEFAULT_CHANNEL_CACHE_EXPIRES: u64 = 300;

#[derive(Clone)]
pub struct ChatServiceImpl {
    pub channel_service: DynChannelService,
    pub queue_service: DynQueueService,
}

impl ChatServiceImpl {
    #[inline]
    pub fn new(
        channel_service: DynChannelService,
        queue_service: DynQueueService,
    ) -> Self {
        Self { channel_service, queue_service }
    }

    #[inline]
    pub fn into_service(self) -> DynChatService {
        Arc::new(self) as DynChatService
    }
}

#[async_trait]
impl ChatService for ChatServiceImpl {}

#[async_trait]
impl CreateQueue for ChatServiceImpl {
    async fn create_queue(
        &self,
        request: CreateQueueRequest,
    ) -> Result<ExecSuccess, ChatServiceError> {
        self.queue_service.create_queue(request).await
    }
}

#[async_trait]
impl RemoveQueue for ChatServiceImpl {
    async fn remove_queue(
        &self,
        query: UserQuery,
    ) -> Result<ExecSuccess, ChatServiceError> {
        self.queue_service.remove_queue(&query).await
    }
}

#[async_trait]
impl GetPublicChannels for ChatServiceImpl {
    async fn get_public_channels(
        &self,
    ) -> Result<GetPublicChannelsResponse, ChatServiceError> {
        let indexes = self.channel_service.channels().indexes.read().await;

        Ok(GetPublicChannelsResponse {
            channels: indexes
                .channel_public
                .values()
                .map(|channel| channel.channel_info())
                .collect(),
        })
    }
}

#[async_trait]
impl AddUserIntoChannel for ChatServiceImpl {
    async fn add_user_into_channel(
        &self,
        request: AddUserIntoChannelRequest,
    ) -> Result<ExecSuccess, ChatServiceError> {
        let AddUserIntoChannelRequest { channel_query, user_id, platforms } =
            request;
        let platforms = Platform::from(platforms);

        let channel_query = channel_query
            .ok_or(ChatServiceError::InvalidArgument)?
            .into_channel_query()?;

        let channel = self
            .channel_service
            .add_user(&channel_query, user_id, platforms)
            .await
            .ok_or(ChatServiceError::ChannelNotExists)?;

        if platforms.contains(Platform::Bancho) {
            self.channel_handle_for_bancho_user(&channel, user_id).await;
            self.channel_update_for_bancho(&channel).await;
        }

        Ok(ExecSuccess::default())
    }
}
#[async_trait]
impl RemoveUserPlatformsFromChannel for ChatServiceImpl {
    async fn remove_user_platforms_from_channel(
        &self,
        request: RemoveUserPlatformsFromChannelRequest,
    ) -> Result<ExecSuccess, ChatServiceError> {
        let RemoveUserPlatformsFromChannelRequest {
            channel_query,
            user_id,
            platforms,
        } = request;
        let platforms = Platform::from(platforms);

        let channel_query = channel_query
            .ok_or(ChatServiceError::InvalidArgument)?
            .into_channel_query()?;

        let channel = self
            .channel_service
            .remove_user_platforms(&channel_query, &user_id, platforms)
            .await
            .ok_or(ChatServiceError::ChannelNotExists)?;

        if platforms.contains(Platform::Bancho) {
            self.channel_handle_for_bancho_user(&channel, user_id).await;
            self.channel_update_for_bancho(&channel).await;
        }

        Ok(ExecSuccess::default())
    }
}

#[async_trait]
impl ChannelUpdateForBancho for ChatServiceImpl {
    async fn channel_update_for_bancho(&self, channel: &Arc<Channel>) {
        let channel_update = bancho_packets::server::ChannelInfo::pack(
            channel.name.load().as_str().into(),
            channel
                .description
                .load()
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_default()
                .into(),
            channel.session_count(Platform::Bancho) as i16,
        );

        channel
            .message_queue
            .lock()
            .await
            .push_message(channel_update.into(), None);
    }
}

#[async_trait]
impl ChannelHandleForBanchoUser for ChatServiceImpl {
    async fn channel_handle_for_bancho_user(
        &self,
        channel: &Arc<Channel>,
        user_id: i32,
    ) {
        let user_notify = bancho_packets::server::ChannelJoin::pack(
            channel.name.load().as_str().into(),
        );

        let target = match self
            .queue_service
            .user_sessions()
            .get(&UserQuery::UserId(user_id))
            .await
        {
            Some(target) => target,
            None => {
                todo!("create session (queue) for target")
            },
        };

        target.push_packet(user_notify.into()).await;
    }
}

#[async_trait]
impl RemoveUserFromChannel for ChatServiceImpl {
    async fn remove_user_from_channel(
        &self,
        request: RemoveUserFromChannelRequest,
    ) -> Result<ExecSuccess, ChatServiceError> {
        let RemoveUserFromChannelRequest { channel_query, user_id } = request;

        let channel_query = channel_query
            .ok_or(ChatServiceError::InvalidArgument)?
            .into_channel_query()?;

        let channel = self
            .channel_service
            .remove_user(&channel_query, &user_id)
            .await
            .ok_or(ChatServiceError::ChannelNotExists)?;

        self.channel_handle_for_bancho_user(&channel, user_id).await;
        self.channel_update_for_bancho(&channel).await;

        Ok(ExecSuccess::default())
    }
}

#[async_trait]
impl ProcessBanchoMessage for ChatServiceImpl {
    async fn process_bancho_message(
        &self,
        sender_id: i32,
        message_content: String,
        target: ChatMessageTarget,
    ) -> Result<SendMessageResponse, ChatServiceError> {
        match target {
            ChatMessageTarget::Channel(channel_query) => {
                let channel = match self
                    .channel_service
                    .get_channel(&channel_query)
                    .await
                {
                    Some(channel) => channel,
                    None => {
                        todo!("try load new channel if possible")
                    },
                };

                let msg = bancho_packets::server::SendMessage {
                    sender: "todo: my name is sender".into(),
                    target: channel.name.to_string().into(),
                    content: message_content.into(),
                    sender_id,
                };

                channel
                    .message_queue
                    .lock()
                    .await
                    .push_message(msg.into_packet_data().into(), None);
            },
            ChatMessageTarget::User(user_query) => {
                let target = match self
                    .queue_service
                    .user_sessions()
                    .get(&user_query)
                    .await
                {
                    Some(target) => target,
                    None => {
                        todo!("create session (queue) for target")
                    },
                };

                let target_name = match user_query {
                    UserQuery::UserId(..) | UserQuery::SessionId(..) => {
                        target.username()
                    },
                    UserQuery::Username(target_name)
                    | UserQuery::UsernameUnicode(target_name) => target_name,
                }
                .into();

                let msg = bancho_packets::server::SendMessage {
                    sender: "todo: my name is sender".into(),
                    target: target_name,
                    content: message_content.into(),
                    sender_id,
                };

                target.push_packet(msg.into_packet_data().into()).await;
            },
        }

        Ok(SendMessageResponse { message_id: 0 })
    }
}

#[async_trait]
impl SendMessage for ChatServiceImpl {
    async fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<SendMessageResponse, ChatServiceError> {
        let SendMessageRequest { sender_id, message, target, platforms } =
            request;

        let target = target
            .ok_or(ChatServiceError::InvalidArgument)?
            .into_message_target()?;

        let mut platforms = Platform::from(platforms);

        if platforms.is_none() {
            platforms = Platform::all()
        }

        if platforms.contains(Platform::Bancho) {
            self.process_bancho_message(sender_id, message, target)
                .await
                .unwrap();
        }

        if platforms.contains(Platform::Lazer) {}

        if platforms.contains(Platform::Web) {}

        Ok(SendMessageResponse { message_id: 0 })
    }
}

#[async_trait]
impl PullChatPackets for ChatServiceImpl {
    async fn pull_chat_packets(
        &self,
        query: UserQuery,
    ) -> Result<BanchoPackets, ChatServiceError> {
        let session = match self.queue_service.user_sessions().get(&query).await
        {
            Some(session) => session,
            None => {
                todo!("create queue for user")
            },
        };

        let records = {
            self.channel_service
                .user_channels()
                .read()
                .await
                .get(&session.user_id)
                .cloned()
        };

        let mut data = session.dequeue_all_packets(None).await;

        if records.is_some() {
            if let Some(channel_ids) = records
                .unwrap()
                .platform_channels
                .read()
                .await
                .get(&Platform::Bancho)
                .map(|c| c.iter().copied().collect::<Vec<u64>>())
            {
                let channels = self.channel_service.channels();
                let indexes = channels.read().await;

                let mut channel_read_process =
                    session.extend.channel_read_process.write().await;

                for channel_id in channel_ids {
                    if let Some(channel) = channels.get_channel_inner(
                        &indexes,
                        &ChannelQuery::ChannelId(channel_id),
                    ) {
                        let channel_process =
                            { channel_read_process.get(&channel_id).cloned() };

                        if let Some(ReceivedMessages {
                            messages,
                            last_msg_id,
                        }) =
                            channel.message_queue.lock().await.receive_messages(
                                &session.user_id,
                                &channel_process.unwrap_or(
                                    session.extend.default_read_process,
                                ),
                                None,
                            )
                        {
                            for packet in messages {
                                data.extend(packet);
                            }

                            channel_read_process
                                .insert(channel_id, last_msg_id);
                        }
                    }
                }
            }
        }

        Ok(BanchoPackets { data })
    }
}

#[derive(Clone)]
pub struct ChatServiceRemote {
    pub client: ChatRpcClient<RpcChannel>,
    pub info: Arc<PublicChannelInfo>,
}

impl FromRpcClient for ChatServiceRemote {
    #[inline]
    fn from_client(client: Self::Client) -> Self {
        Self {
            client,
            info: PublicChannelInfo(CachedAtomic::new(U64::new(
                DEFAULT_CHANNEL_CACHE_EXPIRES,
            )))
            .into(),
        }
    }
}

impl RpcClient for ChatServiceRemote {
    type Client = ChatRpcClient<RpcChannel>;

    #[inline]
    fn client(&self) -> Self::Client {
        self.client.clone()
    }
}

impl IntoService<DynChatService> for ChatServiceRemote {
    #[inline]
    fn into_service(self) -> DynChatService {
        Arc::new(self) as DynChatService
    }
}

#[async_trait]
impl ChatService for ChatServiceRemote {}

#[async_trait]
impl CreateQueue for ChatServiceRemote {
    async fn create_queue(
        &self,
        request: CreateQueueRequest,
    ) -> Result<ExecSuccess, ChatServiceError> {
        self.client()
            .create_queue(request)
            .await
            .map_err(ChatServiceError::RpcError)
            .map(|resp| resp.into_inner())
    }
}

#[async_trait]
impl RemoveQueue for ChatServiceRemote {
    async fn remove_queue(
        &self,
        query: UserQuery,
    ) -> Result<ExecSuccess, ChatServiceError> {
        self.client()
            .remove_queue(Into::<RawUserQuery>::into(query))
            .await
            .map_err(ChatServiceError::RpcError)
            .map(|resp| resp.into_inner())
    }
}

#[async_trait]
impl GetPublicChannels for ChatServiceRemote {
    async fn get_public_channels(
        &self,
    ) -> Result<GetPublicChannelsResponse, ChatServiceError> {
        Ok(GetPublicChannelsResponse { channels: self.info.fetch(self).await? })
    }
}

#[async_trait]
impl AddUserIntoChannel for ChatServiceRemote {
    async fn add_user_into_channel(
        &self,
        request: AddUserIntoChannelRequest,
    ) -> Result<ExecSuccess, ChatServiceError> {
        self.client()
            .add_user_into_channel(request)
            .await
            .map_err(ChatServiceError::RpcError)
            .map(|resp| resp.into_inner())
    }
}

#[async_trait]
impl RemoveUserPlatformsFromChannel for ChatServiceRemote {
    async fn remove_user_platforms_from_channel(
        &self,
        request: RemoveUserPlatformsFromChannelRequest,
    ) -> Result<ExecSuccess, ChatServiceError> {
        self.client()
            .remove_user_platforms_from_channel(request)
            .await
            .map_err(ChatServiceError::RpcError)
            .map(|resp| resp.into_inner())
    }
}

#[async_trait]
impl RemoveUserFromChannel for ChatServiceRemote {
    async fn remove_user_from_channel(
        &self,
        request: RemoveUserFromChannelRequest,
    ) -> Result<ExecSuccess, ChatServiceError> {
        self.client()
            .remove_user_from_channel(request)
            .await
            .map_err(ChatServiceError::RpcError)
            .map(|resp| resp.into_inner())
    }
}

#[async_trait]
impl SendMessage for ChatServiceRemote {
    async fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<SendMessageResponse, ChatServiceError> {
        self.client()
            .send_message(request)
            .await
            .map_err(ChatServiceError::RpcError)
            .map(|resp| resp.into_inner())
    }
}

#[async_trait]
impl PullChatPackets for ChatServiceRemote {
    async fn pull_chat_packets(
        &self,
        query: UserQuery,
    ) -> Result<BanchoPackets, ChatServiceError> {
        self.client()
            .pull_chat_packets(Into::<RawUserQuery>::into(query))
            .await
            .map_err(ChatServiceError::RpcError)
            .map(|resp| resp.into_inner())
    }
}

#[derive(Deref)]
pub struct PublicChannelInfo(pub CachedAtomic<Vec<ChannelInfo>>);

#[async_trait]
impl CachedValue for PublicChannelInfo {
    type Context = ChatServiceRemote;
    type Output = Result<Vec<ChannelInfo>, ChatServiceError>;

    #[inline]
    async fn fetch_new(&self, context: &Self::Context) -> Self::Output {
        context
            .client()
            .get_public_channels(GetPublicChannelsRequest {})
            .await
            .map_err(ChatServiceError::RpcError)
            .map(|resp| {
                let GetPublicChannelsResponse { channels } = resp.into_inner();
                context.info.set(Some(channels.clone().into()));
                channels
            })
    }

    #[inline]
    async fn fetch(&self, context: &Self::Context) -> Self::Output {
        Ok(match context.info.get() {
            Some(cached_value) => {
                if !cached_value.expired {
                    cached_value.cache.to_vec()
                } else {
                    self.fetch_new(context)
                        .await
                        .unwrap_or(cached_value.cache.to_vec())
                }
            },
            None => self.fetch_new(context).await?,
        })
    }
}
