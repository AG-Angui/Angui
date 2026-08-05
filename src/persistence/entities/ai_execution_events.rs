use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "ai_execution_events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub execution_id: String,
    pub event_id: i64,
    pub event_type: String,
    pub stage: Option<String>,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::ai_executions::Entity",
        from = "Column::ExecutionId",
        to = "super::ai_executions::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Execution,
}

impl ActiveModelBehavior for ActiveModel {}
