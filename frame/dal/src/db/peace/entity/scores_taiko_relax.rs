//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.5

use super::sea_orm_active_enums::ScoreGrade;
use super::sea_orm_active_enums::ScoreStatus;
use super::sea_orm_active_enums::ScoreVersion;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "scores_taiko_relax")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: i32,
    #[sea_orm(unique)]
    pub score_md5: String,
    pub map_md5: String,
    pub score_version: ScoreVersion,
    pub score: i32,
    #[sea_orm(column_type = "Decimal(Some((6, 2)))")]
    pub accuracy: Decimal,
    pub combo: i32,
    pub mods: i32,
    pub n300: i32,
    pub n100: i32,
    pub n50: i32,
    pub miss: i32,
    pub geki: i32,
    pub katu: i32,
    pub playtime: i32,
    pub perfect: bool,
    pub status: ScoreStatus,
    pub grade: ScoreGrade,
    pub client_flags: i32,
    pub client_version: String,
    pub confidence: Option<i32>,
    pub verified: bool,
    pub invisible: bool,
    pub verify_at: Option<DateTimeWithTimeZone>,
    pub create_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::leaderboard_taiko_relax::Entity")]
    LeaderboardTaikoRelax,
    #[sea_orm(has_many = "super::score_pp_taiko_relax::Entity")]
    ScorePpTaikoRelax,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Users,
}

impl Related<super::leaderboard_taiko_relax::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LeaderboardTaikoRelax.def()
    }
}

impl Related<super::score_pp_taiko_relax::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ScorePpTaikoRelax.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
