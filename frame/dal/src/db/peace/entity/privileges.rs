//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.5

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "privileges")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub name: String,
    pub description: Option<String>,
    pub priority: i16,
    pub creator_id: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_privileges::Entity")]
    UserPrivileges,
}

impl Related<super::user_privileges::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserPrivileges.def()
    }
}

impl Related<super::channels::Entity> for Entity {
    fn to() -> RelationDef {
        super::channel_privileges::Relation::Channels.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::channel_privileges::Relation::Privileges.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
