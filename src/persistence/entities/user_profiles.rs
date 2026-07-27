use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_profiles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,
    pub avatar_reference: Option<String>,
    pub preferences_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
