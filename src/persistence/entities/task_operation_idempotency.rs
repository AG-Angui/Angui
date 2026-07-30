use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "task_operation_idempotency")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub task_id: String,
    pub volunteer_user_id: String,
    pub operation: String,
    pub idempotency_key: String,
    pub request_fingerprint: String,
    pub response_json: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tasks::Entity",
        from = "Column::TaskId",
        to = "super::tasks::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Task,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::VolunteerUserId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Volunteer,
}

impl Related<super::tasks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Task.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Volunteer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
