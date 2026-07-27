use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "clue_attachment_links")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub clue_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub attachment_id: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::clues::Entity",
        from = "Column::ClueId",
        to = "super::clues::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Clue,
    #[sea_orm(
        belongs_to = "super::case_attachments::Entity",
        from = "Column::AttachmentId",
        to = "super::case_attachments::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Attachment,
}

impl ActiveModelBehavior for ActiveModel {}
