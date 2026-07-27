use sea_orm::entity::prelude::*;
use serde::Serialize;

/// Versioned, approved system instructions. Normal business requests never
/// supply these values; future administrative workflows publish them.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "ai_prompt_templates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub purpose: String,
    pub version: String,
    pub system_instruction: String,
    pub status: String,
    pub created_by_user_id: Option<String>,
    pub published_by_user_id: Option<String>,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
