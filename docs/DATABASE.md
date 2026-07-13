# 数据库与迁移规范

## 1. 当前状态

数据库尚未接入代码。本文件记录后续案件、线索、任务和审计数据实现时必须遵守的技术决策，避免不同开发者分别选择 ORM、数据库或迁移方式。

## 2. 技术基线

- ORM：SeaORM。
- 迁移：`sea-orm-migration`。
- 异步运行时：Tokio。
- 本地默认数据库：SQLite。
- 支持的部署数据库：PostgreSQL、MySQL。
- Rust feature 应覆盖 `sqlx-sqlite`、`sqlx-postgres`、`sqlx-mysql` 和项目选定的 TLS/runtime 组合。

业务服务通过 repository/service 边界使用 SeaORM Entity、ActiveModel 和事务。除迁移、经过评审的复杂查询或数据库诊断外，不在路由和业务服务中直接拼接 SQL。

## 3. 迁移目录

计划采用以下结构：

```text
migration/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   └── m0001_create_cases.rs
└── sql/
    ├── sqlite/
    │   ├── up/
    │   │   └── 0001_create_cases.sql
    │   └── down/
    │       └── 0001_drop_cases.sql
    ├── postgres/
    │   ├── up/
    │   │   └── 0001_create_cases.sql
    │   └── down/
    │       └── 0001_drop_cases.sql
    └── mysql/
        ├── up/
        │   └── 0001_create_cases.sql
        └── down/
            └── 0001_drop_cases.sql
```

`sea-orm-migration` 的 Rust 迁移负责注册顺序、选择当前数据库方言、加载对应 SQL、执行事务并向 SeaORM 迁移表记录状态。SQL 文件是必须保留的结构变更记录，不能只在 Rust builder 中生成而不留下可审查脚本。

## 4. 文件命名

迁移文件统一使用：

```text
NNNN_function_name.sql
```

规则：

- `NNNN` 为从 `0001` 开始的四位全局递增编号。
- `function_name` 使用英文小写 snake_case，描述单一结构变化。
- 三种数据库方言对同一逻辑迁移必须使用相同编号。
- `up` 和 `down` 目录中的文件使用相同编号，但功能名分别描述执行动作。
- 一个编号只处理一个可独立理解的功能，不使用 `misc`、`update_db` 等模糊名称。

示例：

```text
0001_create_cases.sql
0002_create_elder_profiles.sql
0003_create_clues.sql
0004_create_audit_events.sql
0005_add_case_status_index.sql
```

## 5. 不可变更规则

迁移进入共享分支或任何部署环境后：

- 不得重排编号。
- 不得复用已经使用的编号。
- 不得删除历史脚本。
- 不得原地修改已执行脚本。
- 不得通过手工修改数据库代替迁移。
- 修正旧迁移的问题必须创建新编号，例如 `0006_fix_clue_source_constraint.sql`。

只有尚未合并、尚未部署且没有其他开发者依赖的本地迁移才允许重写。

## 6. 跨数据库要求

- 主键、时间、布尔值、JSON、枚举、索引和外键必须分别验证三种数据库的语义。
- 业务状态优先使用可验证的字符串值或关联表，避免依赖单一数据库的原生 enum。
- 时间统一存储为 UTC，接口层负责时区展示。
- 表名和字段名使用 snake_case，避免数据库保留字。
- 不依赖 SQLite 宽松类型行为证明 PostgreSQL/MySQL 一定兼容。
- 数据库特有优化必须放入对应方言目录，并在其他方言提供等价实现或明确降级方案。

## 7. 事务与回滚

- 单个迁移在数据库支持时必须放入事务。
- `up` 和 `down` 脚本需要成对提供；确实不可逆时，在 Rust 迁移和文档中明确说明原因，并阻止误回滚。
- 数据迁移与结构迁移放在同一编号时必须保证重复执行不会破坏数据，或由迁移状态严格保证只执行一次。
- 删除字段、表或数据前先完成兼容版本发布和备份策略，不能在一个版本中直接破坏旧应用。

## 8. 验证要求

每次数据库变更至少验证：

1. 从空数据库执行全部 `up` 迁移。
2. 从上一个发布版本升级到当前版本。
3. 执行本次 `down` 后再执行 `up`。
4. SeaORM Entity 与实际表结构一致。
5. 外键、唯一约束、状态约束和关键索引生效。
6. SQLite、PostgreSQL、MySQL 三种方言脚本编号完全一致。

CI 建立后，SQLite 迁移测试必须常驻执行；PostgreSQL 和 MySQL 使用服务容器执行完整迁移测试。

## 9. 配置约定

数据库通过 `DATABASE_URL` 配置，不在代码中硬编码凭据：

```text
sqlite://data/angui.db?mode=rwc
postgres://user:password@localhost:5432/angui
mysql://user:password@localhost:3306/angui
```

示例值只能出现在 `.env.example` 或测试配置中。真实用户名、密码、生产地址和数据库备份不得进入仓库。
