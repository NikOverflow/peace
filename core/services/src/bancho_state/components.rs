use crate::users::{Session, UserIndexes, UserStore};
use bancho_packets::server::{UserPresence, UserStats};
use bitmask_enum::bitmask;
use peace_domain::bancho_state::ConnectionInfo;
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::{Mutex, MutexGuard};
use tools::{
    atomic::{Atomic, AtomicValue, Bool, F32, U32, U64},
    Ulid,
};

pub type PacketData = Vec<u8>;
pub type PacketDataPtr = Arc<Vec<u8>>;

pub type BanchoSession = Session<BanchoExtend>;
pub type SessionIndexes = UserIndexes<BanchoSession>;
pub type UserSessions = UserStore<BanchoSession>;

#[derive(Debug, Clone)]
pub enum Packet {
    Data(PacketData),
    Ptr(PacketDataPtr),
}

impl Default for Packet {
    fn default() -> Self {
        Self::Data(Vec::new())
    }
}

impl Packet {
    pub fn new(data: PacketData) -> Self {
        Self::Data(data)
    }

    pub fn new_ptr(data: PacketData) -> Self {
        Self::Ptr(Arc::new(data))
    }
}

impl IntoIterator for Packet {
    type Item = u8;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Packet::Data(data) => data.into_iter(),
            Packet::Ptr(ptr) => Arc::try_unwrap(ptr)
                .unwrap_or_else(|ptr| (*ptr).clone())
                .into_iter(),
        }
    }
}

impl From<Arc<Vec<u8>>> for Packet {
    fn from(ptr: Arc<Vec<u8>>) -> Self {
        Self::Ptr(ptr)
    }
}

impl From<Vec<u8>> for Packet {
    fn from(data: Vec<u8>) -> Self {
        Self::Data(data)
    }
}

#[rustfmt::skip]
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Primitive, Hash, Serialize, Deserialize)]
pub enum GameMode {
    #[default]
    Standard            = 0,
    Taiko               = 1,
    Fruits              = 2,
    Mania               = 3,

    StandardRelax       = 4,
    TaikoRelax          = 5,
    FruitsRelax         = 6,
    StandardAutopilot   = 8,

    StandardScoreV2     = 12,
}

impl GameMode {
    #[inline]
    pub fn val(&self) -> u8 {
        *self as u8
    }
}

#[rustfmt::skip]
#[derive(Default, Deserialize)]
#[bitmask(u32)]
pub enum Mods {
    #[default]
    NoMod         = 0,
    NoFail        = 1 << 0,
    Easy          = 1 << 1,
    TouchScreen   = 1 << 2,
    Hidden        = 1 << 3,
    HardRock      = 1 << 4,
    SuddenDeath   = 1 << 5,
    DoubleTime    = 1 << 6,
    Relax         = 1 << 7,
    HalfTime      = 1 << 8,
    NightCore     = 1 << 9,
    FlashLight    = 1 << 10,
    Auto          = 1 << 11,
    SpunOut       = 1 << 12,
    AutoPilot     = 1 << 13,
    Perfect       = 1 << 14,
    Key4          = 1 << 15,
    Key5          = 1 << 16,
    Key6          = 1 << 17,
    Key7          = 1 << 18,
    Key8          = 1 << 19,
    FadeIn        = 1 << 20,
    Random        = 1 << 21,
    Cinema        = 1 << 22,
    Target        = 1 << 23,
    Key9          = 1 << 24,
    KeyCoop       = 1 << 25,
    Key1          = 1 << 26,
    Key3          = 1 << 27,
    Key2          = 1 << 28,
    ScoreV2       = 1 << 29,
    Mirror        = 1 << 30,

    KeyMods = Self::Key1.bits
        | Self::Key2.bits
        | Self::Key3.bits
        | Self::Key4.bits
        | Self::Key5.bits
        | Self::Key6.bits
        | Self::Key7.bits
        | Self::Key8.bits
        | Self::Key9.bits,

    ScoreIncrease = Self::Hidden.bits
        | Self::HardRock.bits
        | Self::FadeIn.bits
        | Self::DoubleTime.bits
        | Self::FlashLight.bits,

    SpeedChanging =
        Self::DoubleTime.bits | Self::NightCore.bits | Self::HalfTime.bits,

    StandardOnly = Self::AutoPilot.bits | Self::SpunOut.bits | Self::Target.bits,
    ManiaOnly = Self::Mirror.bits
        | Self::Random.bits
        | Self::FadeIn.bits
        | Self::KeyMods.bits,
}

impl serde::Serialize for Mods {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.bits())
    }
}

#[rustfmt::skip]
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Primitive, Serialize, Deserialize)]
pub enum UserOnlineStatus {
    #[default]
    Idle          = 0,
    Afk           = 1,
    Playing       = 2,
    Editing       = 3,
    Modding       = 4,
    Multiplayer   = 5,
    Watching      = 6,
    Unknown       = 7,
    Testing       = 8,
    Submitting    = 9,
    Paused        = 10,
    Lobby         = 11,
    Multiplaying  = 12,
    Direct        = 13,
}

impl UserOnlineStatus {
    #[inline]
    pub fn val(&self) -> u8 {
        *self as u8
    }
}

#[rustfmt::skip]
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Primitive, Serialize, Deserialize)]
pub enum PresenceFilter {
    #[default]
    None    = 0,
    All     = 1,
    Friends = 2,
}

impl PresenceFilter {
    #[inline]
    pub fn val(&self) -> i32 {
        *self as i32
    }
}

#[derive(Debug, Default, Serialize)]
pub struct ModeStats {
    pub rank: U32,
    pub pp_v2: F32,
    pub accuracy: F32,
    pub total_hits: U32,
    pub total_score: U64,
    pub ranked_score: U64,
    pub playcount: U32,
    pub playtime: U64,
    pub max_combo: U32,
}

#[derive(Debug, Default, Serialize)]
pub struct BanchoStatus {
    pub online_status: Atomic<UserOnlineStatus>,
    pub description: Atomic<String>,
    pub beatmap_id: U32,
    pub beatmap_md5: Atomic<String>,
    pub mods: Atomic<Mods>,
    pub mode: Atomic<GameMode>,
}

impl BanchoStatus {
    #[inline]
    pub fn update_all(
        &self,
        online_status: UserOnlineStatus,
        description: String,
        beatmap_id: u32,
        beatmap_md5: String,
        mods: Mods,
        mode: GameMode,
    ) {
        self.online_status.set(online_status.into());
        self.description.set(description.into());
        self.beatmap_id.set(beatmap_id);
        self.beatmap_md5.set(beatmap_md5.into());
        self.mods.set(mods.into());
        self.mode.set(mode.into());
    }
}

#[derive(Debug, Default, Serialize)]
pub struct UserModeStatSets {
    pub standard: Option<ModeStats>,
    pub taiko: Option<ModeStats>,
    pub fruits: Option<ModeStats>,
    pub mania: Option<ModeStats>,
    pub standard_relax: Option<ModeStats>,
    pub taiko_relax: Option<ModeStats>,
    pub fruits_relax: Option<ModeStats>,
    pub standard_autopilot: Option<ModeStats>,
    pub standard_score_v2: Option<ModeStats>,
}

#[derive(Debug, Clone, Default)]
pub struct BanchoPacketsQueue {
    pub queue: Arc<Mutex<VecDeque<Packet>>>,
}

impl From<Vec<u8>> for BanchoPacketsQueue {
    fn from(packets: Vec<u8>) -> Self {
        Self::new(VecDeque::from([packets.into()]))
    }
}

impl BanchoPacketsQueue {
    #[inline]
    pub fn new(packets: VecDeque<Packet>) -> Self {
        Self { queue: Arc::new(Mutex::new(packets.into())) }
    }

    #[inline]
    pub async fn queued_packets(&self) -> usize {
        self.queue.lock().await.len()
    }

    #[inline]
    pub async fn push_packet(&self, packet: Packet) -> usize {
        let mut queue = self.queue.lock().await;
        queue.push_back(packet);
        queue.len()
    }

    #[inline]
    pub async fn enqueue_packets<I>(&self, packets: I) -> usize
    where
        I: IntoIterator<Item = Packet>,
    {
        let mut queue = self.queue.lock().await;
        queue.extend(packets);
        queue.len()
    }

    #[inline]
    pub async fn dequeue_packet(
        &self,
        queue_lock: Option<&mut MutexGuard<'_, VecDeque<Packet>>>,
    ) -> Option<Packet> {
        match queue_lock {
            Some(queue) => queue.pop_front(),
            None => self.queue.lock().await.pop_front(),
        }
    }

    #[inline]
    pub async fn dequeue_all_packets(
        &self,
        queue_lock: Option<&mut MutexGuard<'_, VecDeque<Packet>>>,
    ) -> Vec<u8> {
        let mut buf = Vec::new();

        #[inline]
        fn dequeue(
            buf: &mut Vec<u8>,
            queue_lock: &mut MutexGuard<'_, VecDeque<Packet>>,
        ) {
            while let Some(packet) = queue_lock.pop_front() {
                buf.extend(packet);
            }
        }

        match queue_lock {
            Some(queue_lock) => dequeue(&mut buf, queue_lock),
            None => dequeue(&mut buf, &mut self.queue.lock().await),
        };

        buf
    }
}

#[derive(Debug, Default, Serialize)]
pub struct BanchoExtend {
    pub client_version: String,
    pub utc_offset: u8,
    pub presence_filter: Atomic<PresenceFilter>,
    pub display_city: bool,
    pub only_friend_pm_allowed: Bool,
    pub bancho_status: BanchoStatus,
    pub mode_stat_sets: UserModeStatSets,
    #[serde(skip_serializing)]
    pub packets_queue: BanchoPacketsQueue,
    /// Information about the user's connection.
    pub connection_info: ConnectionInfo,
    pub notify_index: Atomic<Ulid>,
}

impl BanchoExtend {
    #[inline]
    pub fn new(
        client_version: String,
        utc_offset: u8,
        display_city: bool,
        only_friend_pm_allowed: impl Into<Bool>,
        initial_packets: Option<Vec<u8>>,
        connection_info: ConnectionInfo,
    ) -> Self {
        Self {
            client_version,
            utc_offset,
            display_city,
            only_friend_pm_allowed: only_friend_pm_allowed.into(),
            packets_queue: initial_packets
                .map(BanchoPacketsQueue::from)
                .unwrap_or_default(),
            connection_info,
            ..Default::default()
        }
    }
}

impl Session<BanchoExtend> {
    #[inline]
    pub fn mode_stats(&self) -> Option<&ModeStats> {
        let stats = &self.extends.mode_stat_sets;
        match &self.extends.bancho_status.mode.load().as_ref() {
            GameMode::Standard => stats.standard.as_ref(),
            GameMode::Taiko => stats.taiko.as_ref(),
            GameMode::Fruits => stats.fruits.as_ref(),
            GameMode::Mania => stats.mania.as_ref(),
            GameMode::StandardRelax => stats.standard_relax.as_ref(),
            GameMode::TaikoRelax => stats.taiko_relax.as_ref(),
            GameMode::FruitsRelax => stats.fruits_relax.as_ref(),
            GameMode::StandardAutopilot => stats.standard_autopilot.as_ref(),
            GameMode::StandardScoreV2 => stats.standard_score_v2.as_ref(),
        }
    }

    #[inline]
    pub fn user_info_packets(&self) -> Vec<u8> {
        let mut info = self.user_stats_packet();
        info.extend(self.user_presence_packet());
        info
    }

    #[inline]
    pub fn user_stats_packet(&self) -> Vec<u8> {
        let status = &self.extends.bancho_status;
        let stats = self.mode_stats();

        UserStats::pack(
            self.user_id,
            status.online_status.load().val(),
            status.description.to_string().into(),
            status.beatmap_md5.to_string().into(),
            status.mods.load().bits(),
            status.mode.load().val(),
            status.beatmap_id.val() as i32,
            stats.map(|s| s.ranked_score.val()).unwrap_or_default() as i64,
            stats.map(|s| s.accuracy.val()).unwrap_or_default(),
            stats.map(|s| s.playcount.val()).unwrap_or_default() as i32,
            stats.map(|s| s.total_score.val()).unwrap_or_default() as i64,
            stats.map(|s| s.rank.val()).unwrap_or_default() as i32,
            stats.map(|s| s.pp_v2.val() as i16).unwrap_or_default(),
        )
    }

    #[inline]
    pub fn user_presence_packet(&self) -> Vec<u8> {
        UserPresence::pack(
            self.user_id,
            self.username.to_string().into(),
            self.extends.utc_offset,
            0, // todo
            1, // todo
            self.extends.connection_info.location.longitude as f32,
            self.extends.connection_info.location.latitude as f32,
            self.mode_stats().map(|s| s.rank.val()).unwrap_or_default() as i32,
        )
    }
}
