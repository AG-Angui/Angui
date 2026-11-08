use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "knowledge_import_batches")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub knowledge_base_id: String,
    pub file_name: String,
    pub source_metadata_json: Json,
    pub status: String,
    pub total_rows: i32,
    pub valid_rows: i32,
    pub invalid_rows: i32,
    pub created_by_user_id: String,
    pub confirmed_by_user_id: Option<String>,
    pub created_at: String,
    pub confirmed_at: Option<String>,
    pub updated_at: String,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DbBackend, QueryTrait, Set, sea_query::Value};
    use serde_json::json;

    #[test]
    fn postgres_source_metadata_uses_json_binding() {
        let metadata = json!({"version":"1.0", "fields":{"note":"Source note"}});
        let statement = Entity::insert(ActiveModel {
            source_metadata_json: Set(metadata.clone()),
            ..Default::default()
        })
        .build(DbBackend::Postgres);
        assert!(
            statement
                .values
                .unwrap()
                .0
                .into_iter()
                .any(|value| { matches!(value, Value::Json(Some(value)) if *value == metadata) })
        );
    }
}
