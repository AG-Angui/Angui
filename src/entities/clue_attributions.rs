use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "clue_attributions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub clue_id: String,
    pub submitted_by_user_id: Option<String>,
    pub reviewed_by_user_id: Option<String>,
    pub reviewed_at: Option<String>,
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
        belongs_to = "super::users::Entity",
        from = "Column::SubmittedByUserId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Submitter,
}

impl Related<super::clues::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Clue.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
