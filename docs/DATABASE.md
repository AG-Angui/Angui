# 数据库与迁移规范

## 1. 当前状态

数据库纵向闭环已经接入代码：SeaORM Entity、服务层事务、认证/RBAC、案件/线索 API 和 `sea-orm-migration` workspace crate 已经落地。SQLite 已完成实际迁移和权限业务联调；PostgreSQL、MySQL 已完成驱动编译支持、方言 SQL 留档和 CI 服务容器配置。

## 2. 技术基线

- ORM：SeaORM 1.1 稳定版本线（锁文件当前解析为 1.1.20）。
- 迁移：`sea-orm-migration`。
- 异步运行时：Tokio。
- 本地默认数据库：SQLite。
- 支持的部署数据库：PostgreSQL、MySQL。
- Rust feature 应覆盖 `sqlx-sqlite`、`sqlx-postgres`、`sqlx-mysql` 和项目选定的 TLS/runtime 组合。

业务服务通过 repository/service 边界使用 SeaORM Entity、ActiveModel 和事务。除迁移、经过评审的复杂查询或数据库诊断外，不在路由和业务服务中直接拼接 SQL。

## 3. 迁移目录

当前采用以下结构：

```text
migration/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── m0001_create_cases.rs
│   ├── m0002_create_elder_profiles.rs
│   ├── m0003_create_clues.rs
│   ├── m0004_create_audit_events.rs
│   ├── m0005_create_users.rs
│   ├── m0006_create_auth_sessions.rs
│   ├── m0007_create_case_memberships.rs
│   ├── m0008_create_clue_attributions.rs
│   └── m0009_add_learner_role.rs
└── sql/
    ├── sqlite/
    │   ├── up/
    │   │   ├── 0001_create_cases.sql
    │   │   ├── 0002_create_elder_profiles.sql
    │   │   ├── 0003_create_clues.sql
    │   │   └── 0004_create_audit_events.sql ... 0009_add_learner_role.sql
    │   └── down/
    │       └── 0001_drop_cases.sql ... 0009_remove_learner_role.sql
    ├── postgres/
    │   ├── up/
    │   │   └── 与 SQLite 相同编号的 8 个 PostgreSQL up 脚本
    │   └── down/
    │       └── 与 SQLite 相同编号的 8 个 PostgreSQL down 脚本
    └── mysql/
        ├── up/
        │   └── 与 SQLite 相同编号的 8 个 MySQL up 脚本
        └── down/
            └── 与 SQLite 相同编号的 8 个 MySQL down 脚本
```

`sea-orm-migration` 的 Rust 迁移负责注册顺序、选择当前数据库方言、加载对应 SQL、执行事务并向 SeaORM 迁移表记录状态。SQL 文件是必须保留的结构变更记录，不能只在 Rust builder 中生成而不留下可审查脚本。

当前共保留 48 个业务 SQL 文件：8 个逻辑迁移 × 3 种方言 × `up/down` 两个方向。脚本使用 `-- statement-break` 作为显式语句边界，由 Rust wrapper 顺序执行。

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
0005_create_users.sql
0006_create_auth_sessions.sql
0007_create_case_memberships.sql
0008_create_clue_attributions.sql
0009_add_learner_role.sql
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

当前自动化测试已经覆盖 SQLite 内存数据库的 `up -> down -> up`，并覆盖账号初始化、登录/登出撤销、案件成员授权、线索可见性裁剪、人工审核和案件状态流转。PostgreSQL/MySQL 由 CI 服务容器执行 `up -> status -> refresh -> status`，仍需以实际 GitHub Actions 结果作为最终证据。

## 9. 配置约定

数据库通过 `DATABASE_URL` 配置，不在代码中硬编码凭据：

```text
sqlite://data/angui.db?mode=rwc
postgres://user:password@localhost:5432/angui
mysql://user:password@localhost:3306/angui
```

示例值只能出现在 `.env.example` 或测试配置中。真实用户名、密码、生产地址和数据库备份不得进入仓库。

应用不会在启动时自动执行迁移。开发者和部署流程必须显式执行：

```powershell
$env:DATABASE_URL = "sqlite://data/angui.db?mode=rwc"
npm run migrate:up
npm run migrate:status
```

需要回滚最近一批迁移时使用 `npm run migrate:down`。共享环境执行 down 前必须确认备份、兼容版本和数据影响。

## 10. 当前表与状态

- `cases`：案件编号、`active/resolved/closed` 状态和时间。
- `elder_profiles`：与案件一对一的老人展示资料；当前字段用于模拟数据，尚未建立字段级权限与加密。
- `clues`：线索内容和审核状态；新线索固定进入 `pending_review`。
- `audit_events`：记录案件创建、案件状态变化、线索提交和人工审核。
- `users`：邮箱、展示名、全局角色、账号状态和 Argon2id 密码哈希。
- `auth_sessions`：只保存令牌 SHA-256 哈希、有效期、撤销时间和最后使用时间。
- `case_memberships`：案件、用户、案件内角色和授权创建者；案件与用户组合唯一。
- `clue_attributions`：线索提交人、审核人和审核时间；兼容认证接入前的历史线索，提交人可以为空。

人工审核可将线索设为 `needs_verification`、`confirmed`、`rejected`、`expired` 或 `duplicate`。未来 AI 只能创建草稿或 `pending_review`，不得直接写入 `confirmed`。

首版将 UUID 和 UTC 时间分别存为字符串 UUID、RFC3339 字符串，以保持三种数据库的统一 Entity。若后续采用 PostgreSQL UUID/TIMESTAMPTZ 或其他原生类型，必须通过新编号迁移演进，不能修改既有脚本。

## 11. 身份数据约束

- 密码原文和会话令牌原文不得写入数据库、审计日志或普通应用日志。
- 邮箱在写入和登录查询前统一去除首尾空白并转为小写。
- 账号删除不是当前能力；禁用账号使用 `status=disabled`，认证时立即拒绝。
- 管理员全局角色不构成案件成员关系，不能绕过 `case_memberships` 读取业务数据。
- 显式运行 `angui-admin bootstrap-demo` 会创建或更新五个 `.invalid` 演示账号（家属、指挥、志愿者、新人、管理员），并撤销这些账号之前的活动会话。该命令仅在 `ANGUI_RUNTIME_ENV` 为 `development`、`preview` 或 `test`，且 `ANGUI_ALLOW_DEMO_BOOTSTRAP=1` 时允许执行。`learner` 与 `admin` 都不是案件成员角色；后者也不能因全局管理员身份绕过 `case_memberships` 读取案件。

## 12. 问询会话

`intake_sessions` stores the family-created, unconfirmed intake draft before a case exists. It has a required creator, lifecycle status, structured answers JSON, timestamps, and an optional unique `case_id` for the later confirmation flow. The foreign key to the creator protects ownership; the optional case relation supports later visibility for that case's authorized commanders and prevents a session from being associated with more than one case.

`intake_question_definitions` is the versioned, database-managed rule set for the initial questionnaire. Each active row provides the stable answer field code, prompt, display order, required marker, and per-question `max_answer_chars`. Sessions snapshot the selected `question_set_version` at creation so later configuration changes cannot re-label the rules that produced an existing draft. The seeded version `1` contains the initial eight questions.

The current creation API writes only `collecting`. The migration also reserves `ready_for_confirmation`, `confirmed`, and `closed` for the follow-on answer and confirmation APIs. Sensitive raw answers remain in the session record and are deliberately excluded from audit metadata and ordinary logs. The database limit is intersected with the server-only `ANGUI_INTAKE_ANSWER_HARD_MAX` safety cap; database configuration cannot increase that hard limit.
