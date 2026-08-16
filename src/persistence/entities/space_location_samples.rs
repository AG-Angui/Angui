use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "space_location_samples")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub space_id: String,
    pub user_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_meters: f64,
    pub captured_at: String,
    pub operation_id: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::collaboration_spaces::Entity",
        from = "Column::SpaceId",
        to = "super::collaboration_spaces::Column::Id"
    )]
    Space,
}
impl Related<super::collaboration_spaces::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Space.def()
    }
}
impl ActiveModelBehavior for ActiveModel {}
