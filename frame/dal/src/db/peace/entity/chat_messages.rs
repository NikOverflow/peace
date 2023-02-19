//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "chat_messages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub sender_id: i32,
    pub channel_id: i64,
    pub timestamp: DateTimeWithTimeZone,
    #[sea_orm(column_type = "Text")]
    pub content_string: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub content_html: Option<String>,
    pub is_action: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::channels::Entity",
        from = "Column::ChannelId",
        to = "super::channels::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Channels,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::SenderId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Users,
}

impl Related<super::channels::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Channels.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
