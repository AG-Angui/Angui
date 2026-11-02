use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "locations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub case_id: String,
    pub location_text: String,
    pub location_precision: String,
    pub location_kind: Option<String>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub location_radius_meters: Option<f64>,
    pub status: String,
    pub merged_into_id: Option<String>,
    pub created_by_user_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::cases::Entity",
        from = "Column::CaseId",
        to = "super::cases::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Case,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::CreatedByUserId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    CreatedBy,
    #[sea_orm(
        belongs_to = "super::locations::Entity"
        from = "Column::MergedIntoId",
        to = "super::locations::Column::Id"
    )]
    MergedInto,
}

impl Related<super::cases::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Case.def()
    }
}
impl ActiveModelBehavior for ActiveModel {}
