use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ElderProfiles::Table)
                    .add_column(
                        ColumnDef::new(ElderProfiles::MobilityNotes)
                            .json_binary()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ElderProfiles::Table)
                    .add_column(
                        ColumnDef::new(ElderProfiles::TransportationAbility)
                            .json_binary()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ElderProfiles::Table)
                    .add_column(
                        ColumnDef::new(ElderProfiles::FrequentLocations)
                            .json_binary()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ElderProfiles::Table)
                    .add_column(
                        ColumnDef::new(ElderProfiles::BehaviorHabits)
                            .json_binary()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ElderProfiles::Table)
                    .add_column(
                        ColumnDef::new(ElderProfiles::SuspiciousMotive)
                            .json_binary()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ElderProfiles::Table)
                    .drop_column(ElderProfiles::SuspiciousMotive)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ElderProfiles::Table)
                    .drop_column(ElderProfiles::BehaviorHabits)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ElderProfiles::Table)
                    .drop_column(ElderProfiles::FrequentLocations)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ElderProfiles::Table)
                    .drop_column(ElderProfiles::TransportationAbility)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ElderProfiles::Table)
                    .drop_column(ElderProfiles::MobilityNotes)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ElderProfiles {
    Table,
    MobilityNotes,
    TransportationAbility,
    FrequentLocations,
    BehaviorHabits,
    SuspiciousMotive,
}
