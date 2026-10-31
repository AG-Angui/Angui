use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "case_map_area_vertices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub area_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub vertex_order: i32,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::case_map_areas::Entity",
        from = "Column::AreaId",
        to = "super::case_map_areas::Column::Id",
        on_delete = "Cascade"
    )]
    Area,
}

impl Related<super::case_map_areas::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Area.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
