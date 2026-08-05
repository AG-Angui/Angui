use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "intake_session_photos")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub session_id: String,
    pub storage_key: String,
    pub original_filename: String,
    pub content_type: String,
    pub byte_size: i64,
    pub sha256: String,
    pub created_by_user_id: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::intake_sessions::Entity",
        from = "Column::SessionId",
        to = "super::intake_sessions::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Session,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::CreatedByUserId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Creator,
}

impl ActiveModelBehavior for ActiveModel {}
