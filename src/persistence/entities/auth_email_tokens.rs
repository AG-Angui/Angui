use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "auth_email_tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub access_request_id: Option<String>,
    pub user_id: Option<String>,
    pub purpose: String,
    pub token_hash: String,
    pub expires_at: String,
    pub consumed_at: Option<String>,
    pub created_at: String,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
