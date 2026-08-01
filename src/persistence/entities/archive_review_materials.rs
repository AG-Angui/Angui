use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "archive_review_materials")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub case_id: String,
    pub version: i32,
    pub parent_material_id: Option<String>,
    pub content: String,
    pub source_scope_json: String,
    pub status: String,
    pub created_by_user_id: String,
    pub reviewed_by_user_id: Option<String>,
    pub reviewed_at: Option<String>,
    pub review_reason: Option<String>,
    pub created_at: String,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
