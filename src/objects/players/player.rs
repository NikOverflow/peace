#![allow(dead_code)]
use super::PlayerBase;

use crate::{
    constants::ClientInfo,
    types::{Location, PacketData},
};
use crate::{
    constants::{BanchoPrivileges, Privileges},
    database::Database,
};

use actix_web::web::Data;
use async_std::sync::{Mutex, RwLock};
use chrono::prelude::{DateTime, Local};
use hashbrown::HashSet;
use queue::Queue;
use tokio_postgres::types::ToSql;

#[derive(Debug)]
pub struct Stats {
    pub rank: i32,
}

#[repr(u8)]
#[derive(Debug)]
pub enum Action {
    Idle,
    Afk,
    Playing,
    Editing,
    Modding,
    Multiplayer,
    Watching,
    Unknown,
    Testing,
    Submitting,
    Paused,
    Lobby,
    Multiplaying,
    OsuDirect,
    None,
}

#[derive(Debug)]
pub struct Player {
    pub id: i32,
    pub name: String,
    pub privileges: i32,
    pub bancho_privileges: i32,
    pub friends: Vec<i32>,
    pub country: String,
    pub ip: String,
    pub address_id: i32,
    pub address_similarity: i32,
    pub only_friend_pm_allowed: bool,
    pub display_city: bool,
    pub osu_version: String,
    pub utc_offset: u8,
    pub location: Location,
    pub stats: Stats,
    pub queue: Mutex<Queue<PacketData>>,
    pub login_time: DateTime<Local>,
    pub login_record_id: i32,
    pub last_active_time: DateTime<Local>,
}

impl Player {
    pub fn to_string(&self) -> String {
        format!("{:?}", self)
    }

    pub async fn from_base(
        base: PlayerBase,
        client_info: ClientInfo,
        ip: String,
        address_id: i32,
        address_similarity: i32,
    ) -> Self {
        let now_time = Local::now();

        Player {
            id: base.id,
            name: base.name,
            privileges: base.privileges,
            bancho_privileges: Player::bancho_privileges(base.privileges),
            friends: vec![base.id],
            country: base.country,
            ip,
            address_id,
            address_similarity,
            only_friend_pm_allowed: client_info.only_friend_pm_allowed,
            display_city: client_info.display_city,
            osu_version: client_info.osu_version,
            utc_offset: client_info.utc_offset as u8,
            location: (0.0, 0.0),
            stats: Stats { rank: 1 },
            queue: Mutex::new(Queue::new()),
            login_time: now_time,
            login_record_id: -1,
            last_active_time: now_time,
        }
    }

    pub fn bancho_privileges(privileges: i32) -> i32 {
        let mut bancho_priv = 0;

        if Privileges::Normal.enough(privileges) {
            // all players have in-game "supporter".
            // this enables stuff like osu!direct,
            // multiplayer in cutting edge, etc.
            bancho_priv |= BanchoPrivileges::Player as i32 | BanchoPrivileges::Supporter as i32
        }

        if Privileges::Mod.enough(privileges) {
            bancho_priv |= BanchoPrivileges::Moderator as i32
        }

        if Privileges::Admin.enough(privileges) {
            bancho_priv |= BanchoPrivileges::Developer as i32
        }

        if Privileges::Dangerous.enough(privileges) {
            bancho_priv |= BanchoPrivileges::Owner as i32
        }

        bancho_priv
    }

    #[inline(always)]
    pub fn update_active(&mut self) {
        self.last_active_time = Local::now();
    }

    pub async fn update_friends_from_database(&mut self, database: &Data<Database>) {
        match database
            .pg
            .query(
                r#"SELECT "friend_id" FROM "user"."friends" WHERE "user_id" = $1;"#,
                &[&self.id],
            )
            .await
        {
            Ok(rows) => {
                let mut friends = vec![self.id];
                friends.extend::<Vec<i32>>(rows.iter().map(|row| row.get("friend_id")).collect());
                self.friends = friends;
            }
            Err(err) => error!(
                "error when update_friends_from_database; user: {}({}); err: {:?}",
                self.name, self.id, err
            ),
        };
    }

    pub async fn update_login_record(&mut self, database: &Data<Database>) {
        self.login_record_id = match database
            .pg
            .query_first(
                r#"INSERT INTO "user_records"."login" (
                    "user_id",
                    "address_id",
                    "ip",
                    "version",
                    "similarity"
                 ) VALUES ($1, $2, $3, $4, $5) RETURNING "id";"#,
                &[
                    &self.id,
                    &self.address_id,
                    &self.ip,
                    &self.osu_version,
                    &self.address_similarity,
                ],
            )
            .await
        {
            Ok(row) => row.get("id"),
            Err(err) => {
                error!(
                    "failed to insert user {}({})'s login record, error: {:?}",
                    self.name, self.id, err
                );
                -1
            }
        };
    }

    #[inline(always)]
    /// Enqueue a packet into queue, returns the length of queue
    pub async fn enqueue(&self, packet_data: PacketData) -> Result<usize, ()> {
        self.queue.lock().await.queue(packet_data)
    }

    #[inline(always)]
    pub async fn dequeue(&self) -> Option<PacketData> {
        self.queue.lock().await.dequeue()
    }

    #[inline(always)]
    /// Get the queue data as vec, readonly
    pub async fn queue_data(&self) -> Vec<PacketData> {
        self.queue.lock().await.vec().clone()
    }

    #[inline(always)]
    /// Get the queue size
    pub async fn queue_len(&self) -> usize {
        self.queue.lock().await.len()
    }

    #[inline(always)]
    pub async fn queue_peek(&self) -> Option<PacketData> {
        self.queue.lock().await.peek()
    }

    #[inline(always)]
    pub async fn queue_is_empty(&self) -> bool {
        self.queue.lock().await.is_empty()
    }
}
