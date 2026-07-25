use sea_orm_migration::prelude::*;

use crate::ensure_rollback_is_safe;

pub struct Migration;

#[derive(DeriveIden)]
enum CasePlaces {
    Table,
    Id,
    CaseId,
    Name,
    PlaceType,
    Address,
    Longitude,
    Latitude,
    Source,
    Visibility,
    ReviewStatus,
    CreatedByUserId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum CaseAttachments {
    Table,
    Id,
    CaseId,
    StorageKey,
    OriginalFilename,
    ContentType,
    ByteSize,
    Sha256,
    Source,
    ReviewStatus,
    CreatedByUserId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Cases {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0018_create_case_places_and_attachments"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CasePlaces::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CasePlaces::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CasePlaces::CaseId).string().not_null())
                    .col(ColumnDef::new(CasePlaces::Name).string().not_null())
                    .col(ColumnDef::new(CasePlaces::PlaceType).string().not_null())
                    .col(ColumnDef::new(CasePlaces::Address).string().not_null())
                    .col(ColumnDef::new(CasePlaces::Longitude).double().null())
                    .col(ColumnDef::new(CasePlaces::Latitude).double().null())
                    .col(ColumnDef::new(CasePlaces::Source).string().not_null())
                    .col(ColumnDef::new(CasePlaces::Visibility).string().not_null())
                    .col(ColumnDef::new(CasePlaces::ReviewStatus).string().not_null())
                    .col(
                        ColumnDef::new(CasePlaces::CreatedByUserId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CasePlaces::CreatedAt).string().not_null())
                    .col(ColumnDef::new(CasePlaces::UpdatedAt).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_case_places_case")
                            .from(CasePlaces::Table, CasePlaces::CaseId)
                            .to(Cases::Table, Cases::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_case_places_creator")
                            .from(CasePlaces::Table, CasePlaces::CreatedByUserId)
                            .to(Users::Table, Users::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_case_places_case_id")
                    .table(CasePlaces::Table)
                    .col(CasePlaces::CaseId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(CaseAttachments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CaseAttachments::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CaseAttachments::CaseId).string().not_null())
                    .col(
                        ColumnDef::new(CaseAttachments::StorageKey)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(CaseAttachments::OriginalFilename)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CaseAttachments::ContentType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CaseAttachments::ByteSize)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CaseAttachments::Sha256).string().not_null())
                    .col(ColumnDef::new(CaseAttachments::Source).string().not_null())
                    .col(
                        ColumnDef::new(CaseAttachments::ReviewStatus)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CaseAttachments::CreatedByUserId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CaseAttachments::CreatedAt)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CaseAttachments::UpdatedAt)
                            .string()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_case_attachments_case")
                            .from(CaseAttachments::Table, CaseAttachments::CaseId)
                            .to(Cases::Table, Cases::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_case_attachments_creator")
                            .from(CaseAttachments::Table, CaseAttachments::CreatedByUserId)
                            .to(Users::Table, Users::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_case_attachments_case_id")
                    .table(CaseAttachments::Table)
                    .col(CaseAttachments::CaseId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[
                ("case places exist", "SELECT 1 FROM case_places LIMIT 1"),
                (
                    "case attachments exist",
                    "SELECT 1 FROM case_attachments LIMIT 1",
                ),
            ],
        )
        .await?;
        manager
            .drop_table(Table::drop().table(CaseAttachments::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CasePlaces::Table).to_owned())
            .await
    }
}
