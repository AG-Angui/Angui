use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "access_requests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub requested_role: String,
    pub status: String,
    pub email_verified_at: Option<String>,
    pub reviewed_by_user_id: Option<String>,
    pub reviewed_at: Option<String>,
    pub review_reason: Option<String>,
    pub created_user_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
