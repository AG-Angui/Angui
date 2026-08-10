use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "message_deliveries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub channel: String,
    pub template: String,
    pub subject_type: String,
    pub subject_id: String,
    pub status: String,
    pub attempt_count: i32,
    pub failure_reason: Option<String>,
    pub created_at: String,
    pub delivered_at: Option<String>,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
