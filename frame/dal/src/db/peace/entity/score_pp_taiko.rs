//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.5

use super::sea_orm_active_enums::PpVersion;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "score_pp_taiko")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub score_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub pp_version: PpVersion,
    #[sea_orm(column_type = "Decimal(Some((16, 2)))")]
    pub pp: Decimal,
    pub raw_pp: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::scores_taiko::Entity",
        from = "Column::ScoreId",
        to = "super::scores_taiko::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    ScoresTaiko,
}

impl Related<super::scores_taiko::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ScoresTaiko.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
