# 安归 Issue 规划草案（按产品模块、API 与三端功能拆分）

> 状态：**已发布到 GitHub**。本草案仍作为 Issue 内容、范围和验收标准的本地来源；
> 实际远程编号见本文末尾的“GitHub Issue 编号映射”。
>
> 依据：`安归_初赛项目计划书_v1.docx`。本草案将计划书中的完整产品愿景拆成可追踪 Issue，并按初赛 15 天交付优先级划分。所有演示数据、图片、聊天记录、位置与轨迹均必须为虚构或充分脱敏的数据。

## 1. 发布规则：标题、GitHub 原生 Type、Priority 与官方 Labels

### 1.1 标题规则

```text
[API] METHOD /api/... - 能力名称
[Feature] 端名/页面 - 用户可见能力
[Data] 数据对象/迁移 - 数据层能力
[Quality] 场景 - 测试、契约或安全验证
[Delivery] 交付物 - Demo、录屏、文档或数据包
```

### 1.2 GitHub 原生 Type

不创建 `type:*` 自定义 Label。发布时使用仓库界面已经提供的 Type：

| 草案标题前缀 | GitHub Type | 说明 |
| --- | --- | --- |
| `[API]`、`[Feature]` | `Feature` | 为产品增加可见的新业务能力，包括 API 与三端页面 |
| `[Data]`、`[Quality]`、`[Delivery]` | `Task` | 迁移、测试、契约、数据准备、录屏和文章等具体工作 |
| 后续发现的回归/缺陷 | `Bug` | 只在实际发现“预期行为未实现”时创建，不预先把计划工作当 Bug |

### 1.3 官方 Labels 与端别 Labels

不创建 `module:*`、`area:*`、`security`、`p0` 等自定义 Labels。模块、端、隐私约束和测试要求保留在每条 Issue 正文中，避免标签体系膨胀。

依照当前项目的拆分约定，仓库额外维护且**仅维护**两个自定义端别 Label：`backend` 和 `frontend`。它们不替代 GitHub 原生 Type、Priority 或官方 Label：

- 已发布的总草案 Issue `#2`–`#67` 统一补加 `backend`，保留原有 Type、Priority 与 Label；即使其中包含前端联调/验收说明，也以这批既有 Issue 的后端/API 交付归属为准。
- 由 `FRONTEND_ISSUE_DRAFT.md` 发布的 `FE-*` Issue 统一使用 `frontend`，并保留其 Type、Priority 和 `enhancement` / `documentation` 等官方 Label。
- 后续只有同时实际覆盖两端的独立联调任务，才可同时使用 `backend` 与 `frontend`；必须在正文说明两端各自的验收范围。

除上述端别 Label 外，仅使用仓库已有的官方 Labels，并按以下规则批量设置：

| 情况 | 使用的官方 Label | 说明 |
| --- | --- | --- |
| 所有新增 API、页面、数据能力 | `enhancement` | 对应“new feature or request” |
| OpenAPI、API 文档、Demo 脚本、技术文章、PPT | `documentation` | 文档和交付材料 |
| 实际发现的产品/测试回归 | `bug` | 不用于尚未开发的能力 |
| 需要外部协作者接手的非核心 Issue | `help wanted` | 初始批量创建时默认不打 |
| 需要新人参与且与安全边界无关的小任务 | `good first issue` | 初始批量创建时默认不打 |
| 总草案的既有 Issue `#2`–`#67` | `backend` | 端别归属；保留原有 Label |

因此，计划中的 API/Feature/Data Issue 使用 `enhancement`；Quality Issue 默认不打功能 Label（若它同时更新 API 文档则打 `documentation`）；Delivery Issue 打 `documentation`；总草案已发布的 Issue 另统一加 `backend`。`duplicate`、`invalid`、`question`、`wontfix` 只在 Issue 生命周期中按真实情况使用。

### 1.4 GitHub 原生 Priority Field

不创建 `P0/P1/P2` 自定义 Field。草案中的 P0/P1/P2 仅用于本地规划，在发布时映射到仓库现有的 `Priority` Field：

| 草案优先级 | GitHub Priority | 使用规则 |
| --- | --- | --- |
| `P0 - 初赛必需` | `High` | 15 天内必须完成的核心闭环与质量门槛 |
| `P1 - 演示增强` | `Medium` | P0 稳定后再进入开发的演示增强能力 |
| `P2 - 后续规划` | `Low` | 仅进入 Backlog，不排入本次初赛计划 |
| 紧急阻断 Demo/安全问题 | `Urgent` | 只在实际阻断发生后调整，初始规划不设置 |

Issue 开放/关闭状态直接使用 GitHub 原生 Open/Closed；不创建自定义 `Status`、`Sprint`、`Estimate`、`Demo Step` Fields。每条正文中的阶段、工期和演示步骤只作为开发说明与排期参考。

### 1.5 优先级含义

- **P0**：没有它无法完整演示“家属提交 → 指挥研判/派单 → 志愿者执行/反馈 → 指挥确认”的核心闭环。
- **P1**：能明显增强 AI、地图或路演表现，但 P0 稳定前不应占用核心开发时间。
- **P2**：来自计划书的后续能力，保留为路线图，不承诺进入本次初赛 Demo。

### 1.6 关于下文“标签/字段”文字

为避免重写后续 65 条 Issue 的详细工程说明，下文的 `标签`、`字段`行仅保留为**草案分类与排期信息**，不会在 GitHub 中创建或设置同名自定义 Label/Field。实际批量发布严格按本节的原生 Type、Priority 和官方 Label 映射执行。

---

## 2. 当前基线（创建一个质量门禁 Issue，不直接关闭）

以下端点已经有实现，但尚未形成“每个端点对应可定位测试文件”的完整 API
质量基线。因此需要创建一个**唯一**的质量门禁 Issue：

### B-00 `[Quality] 现有 API 基线验收与 OpenAPI 对齐`

- **GitHub 原生属性**：`Type=Task`，`Priority=High`，`Label=documentation`。
- **范围**：为下表 11 个既有端点补足独立、可定位的 HTTP/API 测试文件；核对
  实际路由、认证、权限、状态码、JSON 响应和错误码与 `docs/openapi.yaml`、
  `docs/API.md` 一致。
- **不在范围**：不重写现有认证、案件或线索业务逻辑；只有测试或契约发现真实缺陷
  时，才另建 `Bug` Issue 修复，而不是在本 Issue 中无限扩大范围。
- **集中式 API 测试模块**：所有 HTTP/API 契约测试统一由唯一的 Cargo 集成测试入口
  `tests/api_contract.rs` 承载；该入口注册 `tests/api/` 下按端点命名的子模块。这样 CI、
  本地执行和 Issue 验收只需要面向一个测试 target，同时仍能由文件名直接定位端点。纯函数、
  配置解析等不启动 HTTP 服务的单元测试可继续贴近 `src/` 源码，避免为“集中”而把实现细节
  暴露成集成测试；本 Issue 的范围是将所有 **HTTP/API** 测试集中到 `tests/`。
- **目标测试目录**：

  ```text
  tests/api_contract.rs                        # 唯一 API 集成测试入口：注册 support 和 api 子模块
  tests/support/mod.rs                         # SQLite 临时库、迁移、演示用户、HTTP client helper
  tests/api/mod.rs                             # 统一声明下列端点测试子模块
  tests/api/health_get.rs                      # GET    /api/health
  tests/api/auth_login_post.rs                 # POST   /api/auth/login
  tests/api/auth_me_get.rs                     # GET    /api/auth/me
  tests/api/auth_logout_post.rs                # POST   /api/auth/logout
  tests/api/cases_list_get.rs                  # GET    /api/cases
  tests/api/cases_create_post.rs               # POST   /api/cases
  tests/api/case_detail_get.rs                 # GET    /api/cases/{case_id}
  tests/api/case_status_patch.rs               # PATCH  /api/cases/{case_id}/status
  tests/api/case_members_post.rs               # POST   /api/cases/{case_id}/members
  tests/api/case_clues_post.rs                 # POST   /api/cases/{case_id}/clues
  tests/api/clue_review_patch.rs               # PATCH  /api/clues/{clue_id}/review
  ```

- **验收标准**：
  - `tests/api_contract.rs` 是唯一的 API 集成测试 target，显式注册 `support` 和
    `api` 模块；不得把新的 HTTP/API 端点测试散落为多个顶层 `tests/*.rs` target 或
    路由源码内测试。
  - 上述 11 个端点各自都有 `tests/api/` 下独立的测试子模块和至少一个成功路径测试；
    测试文件名可直接从端点反查。
  - 所有受认证保护的端点覆盖无 token/无效 token 的 `401`；所有案件资源端点覆盖
    非成员 `404`；适用的角色拒绝路径覆盖 `403`。
  - `POST /api/auth/login` 覆盖成功、错误密码、未知账号同一失败语义及限流；
    `POST /api/auth/logout` 覆盖会话撤销后 token 不再可用。
  - `GET/POST /api/cases` 覆盖成员可见性、创建者角色限制、请求字段校验；
    `GET /api/cases/{case_id}` 覆盖家属/指挥/志愿者的字段与线索裁剪。
  - `PATCH /api/cases/{case_id}/status` 覆盖合法/非法状态迁移、指挥权限和 closed
    案件行为；成员邀请覆盖角色匹配、重复邀请和跨案件限制。
  - 线索创建覆盖固定 `pending_review`、关闭案件冲突和提交者归属；线索审核覆盖
    commander 限制、全部合法审核状态和审核可见性。
  - 每个测试断言状态码、关键响应字段及统一错误 JSON；测试数据仅使用 `.invalid`
    演示账号和虚构数据，日志输出中不得出现密码或 token。
  - `cargo test --test api_contract --all-features --locked` 全部通过，且不依赖开发者机器
    已有的 `data/angui.db`；随后 `cargo test --workspace --all-features --locked` 全部通过。
  - 对照 `docs/openapi.yaml` 检查 11 个 `method + path`、认证要求、成功状态码、主要
    错误码和请求/响应字段；发现偏差则同步修改实现或文档并在 Issue 中记录决定。
  - 只有所有验收项完成、测试结果附在 Issue、并且无未处理的契约偏差时，才将本
    Issue 关闭。

以下已实现端点构成该 Issue 的固定范围：

| 模块 | 已实现端点 |
| --- | --- |
| 健康 | `GET /api/health` |
| 登录会话 | `POST /api/auth/login`、`GET /api/auth/me`、`POST /api/auth/logout` |
| 案件 | `GET/POST /api/cases`、`GET /api/cases/{case_id}`、`PATCH /api/cases/{case_id}/status`、`POST /api/cases/{case_id}/members` |
| 线索 | `POST /api/cases/{case_id}/clues`、`PATCH /api/clues/{clue_id}/review` |

现有后端已具备数据库会话、角色、案件成员关系、案件级 RBAC、线索服务端字段裁剪和审计事件。新 Issue 必须复用这些边界，而不是把权限判断放到前端。

---

# A. 认证与账号模块

## A1. 登录与会话子模块

### A-01 `[Quality] 登录、会话与退出 API 基线验收`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 1-3`，`Estimate=0.5d`，`Demo Step=基础设施`。
- **标签**：`type:quality`、`module:auth`、`area:backend`、`security`、`testing`。
- **内容**：核对现有 `POST /api/auth/login`、`GET /api/auth/me`、`POST /api/auth/logout` 与 `docs/openapi.yaml`、前端登录流程及演示账号初始化流程的一致性。
- **验收标准**：
  - 有效账号可登录、刷新页面后仍可恢复会话、退出后 token 立即失效。
  - 错误密码不泄露账号是否存在；连续失败触发当前进程内限流。
  - 日志、响应和测试输出不包含密码、Bearer token 或真实隐私数据。
  - 四个核心演示角色账号（家属、指挥、志愿者、新人）均能按文档登录；另有平台管理员账号可登录，但不因全局角色自动获得案件成员权限。

### A-02 `[API] POST /api/auth/password-reset/request - 发起账号找回`

- **字段**：`Priority=P2 - 后续规划`，`Sprint=Post-MVP`，`Estimate=1d`，`Demo Step=基础设施`。
- **标签**：`type:api`、`module:auth`、`area:backend`、`security`、`privacy`、`testing`。
- **内容**：接受邮箱并创建一次性、短时、哈希存储的密码重置请求。首版可只返回通用成功响应，邮件发送通过可替换通知适配层处理。
- **验收标准**：
  - 无论邮箱是否存在，响应文案均一致，避免账号枚举。
  - 请求令牌只保存哈希，设置有效期、使用次数和速率限制。
  - 不在仓库中存储邮件密钥、重置链接或明文令牌。
  - 请求、失效和成功使用均产生不含敏感值的审计记录。

### A-03 `[API] POST /api/auth/password-reset/confirm - 使用一次性令牌重设密码`

- **字段**：`Priority=P2 - 后续规划`，`Sprint=Post-MVP`，`Estimate=1d`，`Demo Step=基础设施`。
- **标签**：`type:api`、`module:auth`、`area:backend`、`security`、`testing`。
- **内容**：验证重置令牌和新密码，采用现有 Argon2id 策略更新密码，并撤销该用户全部已有会话。
- **验收标准**：
  - 过期、已使用、错误令牌均不可重放，且不泄露差异细节。
  - 成功重设后旧密码及该账号所有旧 token 均失效。
  - 新密码满足最小复杂度/长度策略；失败不会覆盖原密码。
  - 有令牌重放、过期、会话撤销和错误输入测试。

## A2. 用户画像与账号状态子模块

### A-04 `[API] GET /api/users/me/profile - 获取当前账号个人资料`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=0.5d`，`Demo Step=基础设施`。
- **标签**：`type:api`、`module:user-profile`、`area:backend`、`privacy`、`testing`。
- **内容**：返回当前用户的显示名、角色、所属队伍（如启用）、头像引用和非敏感偏好；不返回其他成员隐私或案件内敏感资料。
- **验收标准**：
  - 仅可访问当前用户自己的 profile。
  - 响应与 `GET /api/auth/me` 的身份字段一致。
  - 未登录请求返回 `401`；未知字段和无效值均被服务器拒绝。

### A-05 `[API] PATCH /api/users/me/profile - 更新当前账号个人资料`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=0.5d`，`Demo Step=基础设施`。
- **标签**：`type:api`、`module:user-profile`、`area:backend`、`privacy`、`testing`。
- **内容**：允许用户更新显示名、头像引用和必要的演示偏好；禁止在此端点修改全局角色、账号状态和案件角色。
- **验收标准**：
  - 用户只能修改白名单字段，角色/状态字段传入即返回验证错误。
  - 更新后 `GET /api/auth/me` 和 profile 页显示一致。
  - 产生最小化审计事件，不记录完整敏感值。

### A-06 `[API] GET /api/admin/users - 管理员查询账号与状态`

- **字段**：`Priority=P2 - 后续规划`，`Sprint=Post-MVP`，`Estimate=1d`，`Demo Step=基础设施`。
- **标签**：`type:api`、`module:admin`、`area:backend`、`security`、`privacy`、`testing`。
- **内容**：管理员分页查看账号、全局角色、账号状态、创建时间和最后会话时间；不返回密码哈希、会话 token 或完整敏感资料。
- **验收标准**：
  - 仅 admin 可调用，且 admin 不因此自动获得案件业务权限。
  - 分页、筛选和排序有白名单限制。
  - 每次列表访问都可追踪，不输出敏感认证数据。

### A-07 `[API] PATCH /api/admin/users/{user_id}/status - 启用、停用或锁定账号`

- **字段**：`Priority=P2 - 后续规划`，`Sprint=Post-MVP`，`Estimate=1d`，`Demo Step=基础设施`。
- **标签**：`type:api`、`module:admin`、`area:backend`、`security`、`testing`。
- **内容**：管理员修改账号状态，停用/锁定时撤销活跃会话；角色变更另行设计，避免与案件成员权限混淆。
- **验收标准**：
  - 状态机明确且不可通过客户端任意字符串绕过。
  - 停用账号的 token 立即不可用，不能创建案件、任务或线索。
  - 记录操作者、前后状态和原因，不记录凭据。

---

# B. 家属端：建档、问询、老人画像与补充材料

## B1. AI 引导问询子模块

### B-01 `[API] POST /api/intake-sessions - 创建家属走失信息问询会话`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 1-3`，`Estimate=1d`，`Demo Step=家属建档`。
- **标签**：`type:api`、`module:ai`、`module:case`、`client:family`、`area:backend`、`ai-human-review`、`demo`、`testing`。
- **内容**：家属开启“老人走失求助”问询会话，保存结构化初始答案和会话状态。问询顺序覆盖基本信息、身体状况、行为习惯、最后出现、常去地点、随身物品、交通能力和后续线索。
- **验收标准**：
  - 仅登录家属可创建；会话只对创建者和后续获授权指挥可见。
  - 输入字段被校验、脱敏策略明确；不允许客户端直接写入 `confirmed` 事实。
  - 返回下一步所需问题或缺失字段，AI 不可用时返回规则化问询问题。
  - 有创建、无权限、字段校验和 AI 降级测试。

### B-02 [API] POST /api/intake-sessions/{session_id}/answers - 提交问询答案并获取下一问

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 1-3`，`Estimate=1d`，`Demo Step=家属建档`。
- **标签**：`type:api`、`module:ai`、`module:case`、`client:family`、`area:backend`、`ai-human-review`、`privacy`、`testing`。
- **内容**：接收家属逐步补充的答案，更新缺失字段清单，并由规则或 Agent 输出下一问。原始自由文本与提取出的候选字段要可区分。
- **验收标准**：
  - 只有会话创建者可追加答案；已关闭会话不可继续写入。
  - 每个 AI 提取字段带来源、状态 `draft`、生成时间和模型/模板版本（若实际调用模型）。
  - AI 输出不作为案件正式事实，必须经过确认步骤。
  - 对提示注入、超长文本、无效会话和重复提交有测试。

### B-03 `[API] GET /api/intake-sessions/{session_id}/profile-draft - 获取老人画像草稿`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 1-3`，`Estimate=0.5d`，`Demo Step=家属建档`。
- **标签**：`type:api`、`module:ai`、`module:user-profile`、`client:family`、`area:backend`、`ai-human-review`、`privacy`、`testing`。
- **内容**：将问询答案整理成标准化老人画像草稿：体貌、衣着、健康注意、行动能力、交通能力、常去地点、最后出现信息、待补充项。
- **验收标准**：
  - 响应明确显示 `draft`、来源范围、生成时间与“需人工确认”提示。
  - 家属只能读取自己的草稿；志愿者无权读取健康/联系类内容。
  - 不以确定语气推断去向，不输出“老人一定在某处”。

### B-04 [API] POST /api/intake-sessions/{session_id}/confirm - 确认画像草稿并创建案件

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 1-3`，`Estimate=1d`，`Demo Step=家属建档`。
- **标签**：`type:api`、`module:case`、`module:ai`、`client:family`、`area:backend`、`area:database`、`ai-human-review`、`security`、`demo`、`testing`。
- **内容**：家属审阅、修改并确认草稿后，以事务方式创建案件、老人画像、家属成员关系及 `case.created` 审计事件。可复用现有创建案件服务，但必须防止重复确认创建多个案件。
- **验收标准**：
  - 确认前草稿不出现在正式案件详情；确认后正式案件有唯一 ID 与初始 `active` 状态。
  - 家属可覆盖错误草稿字段；服务端保存“人工确认”状态而非宣称 AI 已确认。
  - 同一会话重复确认具有幂等行为或明确 `409 Conflict`，不得产生重复案件。
  - 事务回滚时不遗留半成品案件/画像/审计记录。

## B2. 老人资料、地点与附件子模块

### B-05 `[API] PATCH /api/cases/{case_id}/elder-profile - 补充或更正老人画像`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=1d`，`Demo Step=家属建档`。
- **标签**：`type:api`、`module:case`、`module:user-profile`、`client:family`、`client:command`、`area:backend`、`privacy`、`testing`。
- **内容**：家属可补充本人案件资料；指挥可更正必要字段。每次更正保留操作者、时间及前后值审计，不覆盖历史证据。
- **验收标准**：
  - 家属只能修改自己作为成员的案件，且不能改变案件状态、成员关系或内部任务信息。
  - 志愿者不可修改，非成员返回 `404`。
  - 志愿者读取详情时健康注意字段仍由服务器裁剪。

### B-06 `[API] POST /api/cases/{case_id}/places - 添加常去地点或关键地点`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=0.5d`，`Demo Step=家属建档`。
- **标签**：`type:api`、`module:case`、`module:map`、`client:family`、`client:command`、`area:backend`、`map-amap`、`privacy`、`testing`。
- **内容**：保存常去地点、最后出现点之外的关键地点，含名称、类型、文字地址、可选坐标、来源与可见级别；坐标可由后续地理编码补齐。
- **验收标准**：
  - 家属/指挥仅能向自己案件添加地点；志愿者不能添加敏感家庭地址。
  - 经纬度范围、地点类型和文本长度均在服务器验证。
  - `public/confirmed/internal` 可见级别决定后续地图 API 的返回范围。

### B-07 `[API] GET /api/cases/{case_id}/places - 获取按角色裁剪的关键地点`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=0.5d`，`Demo Step=家属建档`。
- **标签**：`type:api`、`module:case`、`module:map`、`client:family`、`client:command`、`client:volunteer`、`area:backend`、`privacy`、`testing`。
- **内容**：返回最后出现点与授权关键地点；不同角色得到不同精度和不同可见级别的数据。
- **验收标准**：
  - 家属只见已确认/公开进展地点；志愿者只见完成本人任务所需地点；指挥见内部地点。
  - 服务端而非前端负责剔除家庭住址、未核实地点和内部搜索方向。
  - 非成员返回 `404`。

### B-08 `[API] POST /api/cases/{case_id}/attachments - 上传案件图片或附件`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=2d`，`Demo Step=家属建档`。
- **标签**：`type:api`、`module:case`、`client:family`、`client:command`、`client:volunteer`、`area:backend`、`area:integration`、`security`、`privacy`、`testing`。
- **内容**：上传老人照片、线索图片或任务反馈图片。首版使用受控本地目录或对象存储适配层，保存元数据、所有者、关联资源和审核状态。
- **验收标准**：
  - 严格限制 MIME 类型、文件大小、数量与访问权限；拒绝可执行文件和伪造 Content-Type。
  - 清除非必要图片 EXIF/GPS 元数据，上传路径不可猜测且不可公开列目录。
  - 图片只经授权下载；家属不读取志愿者内部任务附件。
  - 外部对象存储不可用时有清晰失败响应，不留下孤立记录。

### B-09 `[Feature] 家属端 - 引导问询、画像确认与线索补充`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 1-3`，`Estimate=2d`，`Demo Step=家属建档`。
- **标签**：`type:feature`、`module:case`、`module:ai`、`client:family`、`area:frontend`、`ai-human-review`、`privacy`、`demo`、`testing`。
- **内容**：把“老人走失求助”做成移动优先的分步问询页：填写、追问、查看画像草稿、手工修改/确认、创建案件、后续补充线索与地点。
- **验收标准**：
  - 每一步明确必填项、缺失项和保存状态，不要求家属一次填写完整表单。
  - 老人画像显著标记“AI/规则整理草稿，需人工确认”；用户可编辑或拒绝。
  - 不出现志愿者轨迹、内部任务、未核实线索或后台字段。
  - 在手机宽度可完整演示，不依赖开发者工具或数据库手工写入。

### B-10 `[Feature] 家属端 - 公开案件进展与防走失知识`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=1d`，`Demo Step=家属建档`。
- **标签**：`type:feature`、`module:case`、`module:knowledge`、`client:family`、`area:frontend`、`privacy`、`demo`。
- **内容**：展示经确认的案件进展、待补充问题和防走失知识卡片；公共进展与内部指挥视图不可共用同一完整接口。
- **验收标准**：
  - 只显示家属可见的确认线索和公开摘要。
  - 页面明确“信息以人工审核记录为准”。
  - 可离线展示固定防走失知识内容。

---

# C. 案件与成员协作模块

## C1. 案件生命周期子模块

### C-01 `[API] GET /api/cases - 案件列表 API 基线验收`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 1-3`，`Estimate=0.5d`，`Demo Step=指挥研判`。
- **标签**：`type:quality`、`module:case`、`area:backend`、`security`、`testing`。
- **内容**：验证现有案件列表仅返回当前用户具有成员关系的案件，并按角色输出正确 `access_role`。
- **验收标准**：无成员关系返回空列表；管理员不因全局角色自动看到案件；列表不含健康详情、未核实线索或其他成员隐私。

### C-02 `[API] PATCH /api/cases/{case_id}/status - 案件状态 API 基线验收`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=0.5d`，`Demo Step=指挥研判`。
- **标签**：`type:quality`、`module:case`、`area:backend`、`security`、`testing`。
- **内容**：验收现有 `active/resolved/closed` 状态机、指挥权限、关闭后不可新增线索和审计事务。
- **验收标准**：允许和禁止的迁移符合 OpenAPI；家属/志愿者不能改状态；关闭案件新增线索返回冲突；状态变化有审计记录。

### C-03 `[API] POST /api/cases/{case_id}/members - 案件成员邀请 API 基线验收`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 1-3`，`Estimate=0.5d`，`Demo Step=家属建档`。
- **标签**：`type:quality`、`module:case`、`module:auth`、`area:backend`、`security`、`testing`。
- **内容**：核对家属仅可显式邀请指挥、指挥可添加角色匹配成员、角色与账号全局角色一致的现有规则。
- **验收标准**：不能跨案件添加、不能重复添加、不能把账号伪装成其他角色；所有成功邀请均有审计记录。

### C-04 `[API] GET /api/cases/{case_id}/public-progress - 获取家属可见的公开进展`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=1d`，`Demo Step=指挥研判`。
- **标签**：`type:api`、`module:case`、`client:family`、`area:backend`、`privacy`、`ai-human-review`、`testing`。
- **内容**：为家属单独提供公开进展视图，包含案件状态、已确认进展、需要家属补充的信息和安全/联系提示。
- **验收标准**：不返回未核实线索、内部搜索方向、任务分配、志愿者位置、病史全文或其他成员详情；每项进展有更新时间和审核状态。

### C-05 `[Feature] 指挥端 - 案件总览与成员协作`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=1d`，`Demo Step=指挥研判`。
- **标签**：`type:feature`、`module:case`、`client:command`、`area:frontend`、`security`、`demo`。
- **内容**：完善指挥工作台中的案件列表、案件详情、成员邀请、案件状态切换和角色信息提示。
- **验收标准**：指挥能进入获授权案件并邀请演示志愿者；关闭案件后界面禁用不合法操作；家属/志愿者无法通过路由进入指挥控制页。

---

# D. 线索与人工审核模块

## D1. 线索采集与时间轴子模块

### D-01 `[API] GET /api/cases/{case_id}/clues - 获取角色裁剪的线索时间轴`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=1d`，`Demo Step=指挥研判`。
- **标签**：`type:api`、`module:clue`、`client:family`、`client:command`、`client:volunteer`、`area:backend`、`security`、`privacy`、`testing`。
- **内容**：从现有案件详情中拆出可分页的线索时间轴端点。指挥看全量；家属看已确认和本人提交；志愿者看已确认且完成任务所需的信息。
- **验收标准**：
  - 排序、分页、筛选状态均有白名单且稳定。
  - 服务端角色裁剪与现有案件详情规则一致。
  - 无案件成员关系返回 `404`，而不是暴露线索或案件存在性。

### D-02 `[API] POST /api/cases/{case_id}/clues - 提交线索 API 基线验收`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=0.5d`，`Demo Step=家属建档`。
- **标签**：`type:quality`、`module:clue`、`area:backend`、`security`、`testing`。
- **内容**：验收已有线索提交端点的字段校验、关闭案件限制、固定 `pending_review` 状态、提交人归属和审计事务。
- **验收标准**：客户端不能直接提交 `confirmed`；关闭案件拒绝写入；提交者可识别自己的待审核线索；审计不写入完整敏感内容。

### D-03 `[API] PATCH /api/clues/{clue_id}/review - 人工审核线索 API 基线验收`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=0.5d`，`Demo Step=指挥研判`。
- **标签**：`type:quality`、`module:clue`、`client:command`、`area:backend`、`security`、`ai-human-review`、`testing`。
- **内容**：验收只有案件指挥可审核线索，并以 `needs_verification/confirmed/rejected/expired/duplicate` 更新状态与审核人信息。
- **验收标准**：志愿者和家属无法审核；审核前后状态及操作者可追溯；被拒绝/重复线索不作为公开事实显示。

### D-04 `[API] POST /api/cases/{case_id}/clue-drafts - 将聊天/文本整理为待审核线索草稿`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=1.5d`，`Demo Step=指挥研判`。
- **标签**：`type:api`、`module:clue`、`module:ai`、`client:command`、`area:backend`、`area:integration`、`ai-human-review`、`privacy`、`demo`、`testing`。
- **内容**：把模拟微信群聊、电话记录或家属自由文本传给 AI/规则提取层，产出时间、地点、来源、内容、建议动作和不确定性说明的草稿列表。
- **验收标准**：
  - 输出只能是 `draft/pending_review`，禁止直接创建 `confirmed` 线索。
  - 每条提取结果保留原始输入引用、模型/模板版本（若使用 AI）和不确定性说明。
  - AI 超时/格式异常时可退回规则化人工录入，不阻断案件处理。
  - 指挥须显式确认后才能调用正式线索创建/审核流程。

### D-05 `[API] POST /api/clues/{clue_id}/attachments - 上传线索佐证附件`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=1d`，`Demo Step=志愿者执行`。
- **标签**：`type:api`、`module:clue`、`client:family`、`client:volunteer`、`client:command`、`area:backend`、`security`、`privacy`、`testing`。
- **内容**：将图片/文件挂到具体线索，沿用案件附件的类型、大小、元数据清理与访问控制策略。
- **验收标准**：附件访问需同时满足线索所在案件的成员关系和角色可见性；家属不能读取内部附件；恶意类型与超限文件被拒绝。

### D-06 `[Feature] 指挥端 - 线索时间轴、审核队列与结构化草稿`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=1.5d`，`Demo Step=指挥研判`。
- **标签**：`type:feature`、`module:clue`、`module:ai`、`client:command`、`area:frontend`、`ai-human-review`、`demo`、`testing`。
- **内容**：指挥端展示时间轴、待审核队列、审核按钮、来源与时间地点，并在 P1 接入聊天整理草稿的“确认/修改/拒绝”界面。
- **验收标准**：确认、待核实、排除、重复状态视觉可区分；不显示“AI 已确认”误导文案；审核后的变化立即影响指挥视图和后续摘要。

---

# E. 任务、位置、轨迹与志愿者安全模块

## E1. 任务生命周期子模块

### E-01 `[Data] tasks、task_assignments、task_location_reports - 任务与轨迹数据迁移`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=1d`，`Demo Step=志愿者执行`。
- **标签**：`type:data`、`module:task`、`area:database`、`security`、`privacy`、`testing`。
- **内容**：新增任务、单任务单志愿者分配、模拟位置回传表及三种数据库方言迁移。任务关联案件、可选来源线索、地点、风险、截止时间、状态和结果；位置关联任务与采集时间。
- **验收标准**：
  - SQLite、PostgreSQL、MySQL 均有编号一致的 up/down SQL 与迁移定义。
  - 约束保证任务与案件、分配与用户、位置与任务的关联完整性。
  - 不将真实设备标识、连续后台定位或精确轨迹长期保留作为 MVP 默认行为。
  - 迁移执行 `up -> down -> up` 验证通过。

### E-02 `[API] POST /api/cases/{case_id}/tasks - 创建并分配搜救任务`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=1.5d`，`Demo Step=指挥研判`。
- **标签**：`type:api`、`module:task`、`client:command`、`area:backend`、`area:database`、`security`、`privacy`、`demo`、`testing`。
- **内容**：指挥在案件内创建任务，填入标题、目标、地点文字、可选坐标、截止时间、风险等级、安全提示、可选来源线索及一个志愿者受领人；初始状态为 `assigned`。
- **验收标准**：
  - 仅案件成员中的 commander 可创建；受领人必须是该案件活跃 volunteer。
  - 非成员返回 `404`；家属/志愿者/admin 无案件关系不能创建。
  - 经纬度、风险等级、截止时间、来源线索归属均由服务端校验。
  - 任务、分配和 `task.created/task.assigned` 审计事件在一个事务中提交。

### E-03 `[API] GET /api/cases/{case_id}/tasks - 获取案件任务列表`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=0.5d`，`Demo Step=指挥研判`。
- **标签**：`type:api`、`module:task`、`client:command`、`client:family`、`client:volunteer`、`area:backend`、`security`、`privacy`、`testing`。
- **内容**：返回角色裁剪的案件任务列表：指挥看全量；志愿者只看自己任务；家属不看内部任务，返回空列表或使用独立公开进展接口。
- **验收标准**：不泄露其他志愿者身份、路线、位置和任务细节；稳定排序/分页；服务端执行过滤；非成员 `404`。

### E-04 `[API] GET /api/tasks/mine - 获取当前志愿者的个人任务队列`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=0.5d`，`Demo Step=志愿者执行`。
- **标签**：`type:api`、`module:task`、`client:volunteer`、`area:backend`、`security`、`privacy`、`testing`。
- **内容**：不让志愿者枚举案件 ID，直接返回本人被分配任务及必要案情、地点文本、安全提示和截止时间。
- **验收标准**：志愿者只获取本人任务；家属、指挥和 admin 调用返回 `403`；无任务返回 `200 []`；不返回健康详情、家属联系信息、全量线索或他人轨迹。

### E-05 `[API] PATCH /api/tasks/{task_id}/status - 更新任务执行状态`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=1d`，`Demo Step=志愿者执行`。
- **标签**：`type:api`、`module:task`、`client:command`、`client:volunteer`、`area:backend`、`security`、`testing`。
- **内容**：实现 `assigned -> accepted -> active -> completed` 状态机；受领志愿者仅可推进自己的任务，案件指挥可在未完成前 `cancelled`。
- **验收标准**：允许迁移返回更新任务；非法迁移返回 `409`；取消与完成均为终态；每次成功迁移写入前后状态、操作者和时间的审计事件。

### E-06 `[API] POST /api/tasks/{task_id}/location-reports - 上报任务期间的模拟位置`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=1d`，`Demo Step=志愿者执行`。
- **标签**：`type:api`、`module:task`、`module:map`、`client:volunteer`、`area:backend`、`security`、`privacy`、`demo`、`testing`。
- **内容**：仅允许任务受领人在 `active` 期间上传模拟纬度、经度、精度、采集时间与来源 `simulated`。禁止实现设备后台持续定位。
- **验收标准**：非受领人、未激活、已完成/取消、过期/未来异常时间和越界坐标均被拒绝；审计/普通日志不写精确坐标；位置只通过后续角色裁剪地图视图读取。

### E-07 `[API] POST /api/tasks/{task_id}/feedback - 提交任务结果与关联线索`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=1d`，`Demo Step=志愿者执行`。
- **标签**：`type:api`、`module:task`、`module:clue`、`client:volunteer`、`area:backend`、`ai-human-review`、`testing`。
- **内容**：志愿者提交文字结果、可选附件与位置，服务端创建或关联一条 `pending_review` 线索；不能由反馈直接更改案件事实。
- **验收标准**：只有本人任务受领人可提交；任务/案件状态不允许时被拒绝；指挥审核线索后才影响案情与地图；任务反馈和线索可追溯关联。

## E2. 安全提醒子模块

### E-08 `[API] GET /api/tasks/{task_id}/safety-briefing - 获取任务安全提示`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=0.5d`，`Demo Step=志愿者执行`。
- **标签**：`type:api`、`module:task`、`module:map`、`client:volunteer`、`area:backend`、`security`、`map-amap`、`testing`。
- **内容**：返回任务风险等级、夜间/天气/区域注意事项、紧急停止提示和最后更新时间。首版规则化生成，外部天气失败时仍返回人工安全规则。
- **验收标准**：仅任务受领人和案件指挥可读；不将 AI/天气建议表述为强制现场指挥命令；外部服务失败有文字降级内容。

### E-09 `[Quality] 志愿者任务与位置权限验证 - 越权、状态机与隐私`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 12-15`，`Estimate=1d`，`Demo Step=志愿者执行`。
- **标签**：`type:quality`、`module:task`、`area:backend`、`security`、`privacy`、`testing`。
- **内容**：为任务创建、分配、状态变更、位置回传和反馈补齐角色/案件成员/任务受领人/终态测试矩阵。
- **验收标准**：覆盖 family、commander、受领 volunteer、非受领 volunteer、非成员、admin 的成功与拒绝路径；精确位置不出现在家属/其他志愿者响应和日志；所有状态机冲突可重复验证。

### E-10 `[Feature] 指挥端 - 任务看板、分配与执行状态`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=1.5d`，`Demo Step=指挥研判`。
- **标签**：`type:feature`、`module:task`、`client:command`、`area:frontend`、`security`、`demo`。
- **内容**：实现指挥端任务创建表单、志愿者选择、任务卡片/看板、状态显示、取消操作、关联线索入口与安全提示展示。
- **验收标准**：可把“核实公园北门至菜市场路线”分配给演示志愿者；状态从 assigned 到 completed 可见；错误状态/无权限时不出现假成功；移动端和桌面端均可用。

### E-11 `[Feature] 志愿者端 - 我的任务、状态推进、模拟位置与线索反馈`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 4-7`，`Estimate=1.5d`，`Demo Step=志愿者执行`。
- **标签**：`type:feature`、`module:task`、`module:clue`、`client:volunteer`、`area:frontend`、`security`、`privacy`、`demo`。
- **内容**：替换志愿者占位页，展示个人任务、接受/开始/完成按钮、模拟位置回传表单、安全提示和线索反馈入口。
- **验收标准**：只显示登录志愿者自己的任务；只有 active 状态显示位置表单；明确标注“模拟演示位置”；不展示其他志愿者、内部线索、家庭隐私或完整病史。

---

# F. 地图、高德能力与态势模块

## F1. 地图数据与角色视图子模块

### F-01 `[API] GET /api/cases/{case_id}/map-view - 获取按角色裁剪的地图态势`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 8-11`，`Estimate=1d`，`Demo Step=指挥研判`。
- **标签**：`type:api`、`module:map`、`module:case`、`module:task`、`client:family`、`client:command`、`client:volunteer`、`area:backend`、`security`、`privacy`、`demo`、`testing`。
- **内容**：统一输出最后出现点、授权常去地点、已确认线索点、任务点和简化模拟轨迹。每个点包含类型、显示名、坐标或 null、文本地点、更新时间、可见级别。
- **验收标准**：指挥看案件任务与被授权位置；志愿者只看本人任务和本人轨迹；家属只看公开/已确认点；无坐标时仍提供文字地点；非成员 `404`。

### F-02 `[API] GET /api/cases/{case_id}/pois - 检索案件周边 POI`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=1d`，`Demo Step=指挥研判`。
- **标签**：`type:api`、`module:map`、`client:command`、`client:volunteer`、`area:backend`、`area:integration`、`map-amap`、`privacy`、`testing`。
- **内容**：经高德适配层按案件授权中心点查医院、派出所、公交站、市场、社区服务中心等 POI。调用方不可传任意敏感坐标越权检索。
- **验收标准**：POI 类别白名单、次数限制和超时处理明确；外部 API 失败时返回可演示的固定模拟 POI 或可识别降级状态；不记录 API key/完整查询隐私。

### F-03 `[API] GET /api/tasks/{task_id}/navigation - 获取任务导航链接或路线摘要`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=0.5d`，`Demo Step=志愿者执行`。
- **标签**：`type:api`、`module:map`、`module:task`、`client:volunteer`、`area:backend`、`area:integration`、`map-amap`、`security`、`testing`。
- **内容**：为任务受领志愿者生成到任务点的高德导航链接或路线文本摘要；指挥可查看任务路线摘要，但家属不可获取内部路线。
- **验收标准**：仅受领人/案件指挥可调用；任务无坐标时退回文字地点；链接只指向任务已授权地点；外部路线失败不阻断任务状态操作。

### F-04 `[API] GET /api/tasks/{task_id}/conditions - 获取天气、路况与降级安全信息`

- **字段**：`Priority=P2 - 后续规划`，`Sprint=Post-MVP`，`Estimate=1d`，`Demo Step=志愿者执行`。
- **标签**：`type:api`、`module:map`、`module:task`、`client:volunteer`、`area:integration`、`map-amap`、`security`、`testing`。
- **内容**：通过外部服务返回任务点天气、路况和数据时间；首版禁止把外部数据转化为强制派遣/风险结论。
- **验收标准**：结果携带来源与更新时间；超时/无配额返回规则化安全提示；不含“系统建议必须进入某区域”等危险指令。

### F-05 `[Feature] 指挥端 - 高德/静态地图态势与文字降级`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 8-11`，`Estimate=1.5d`，`Demo Step=指挥研判`。
- **标签**：`type:feature`、`module:map`、`client:command`、`area:frontend`、`area:integration`、`map-amap`、`demo`、`testing`。
- **内容**：地图展示最后出现点、常去地点、确认线索点、任务点、模拟轨迹和 POI。地图 SDK 或网络不可用时保留文字地点列表、任务列表和状态。
- **验收标准**：点位类型有图例且不只依赖颜色区分；接口返回的角色数据决定展示内容；地图失败时核心 Demo 仍能完成；不会把其他志愿者轨迹暴露在家属或志愿者端。

### F-06 `[Feature] 志愿者端 - 一键导航与任务安全提示`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=1d`，`Demo Step=志愿者执行`。
- **标签**：`type:feature`、`module:map`、`module:task`、`client:volunteer`、`area:frontend`、`map-amap`、`security`、`demo`。
- **内容**：任务卡提供导航入口、文字地点、风险提示、天气/路况降级提示和停止回传说明。
- **验收标准**：无地图 key 或导航失败时仍显示地点和人工安全规则；不要求浏览器后台定位授权；用户能理解当前是否在回传模拟位置。

---

# G. AI、公共案情与人工审核模块

## G1. 公共案情总结子模块

### G-01 `[API] GET /api/cases/{case_id}/summary - 获取角色裁剪的公共案情总结`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 8-11`，`Estimate=1d`，`Demo Step=指挥研判`。
- **标签**：`type:api`、`module:ai`、`module:case`、`module:clue`、`module:task`、`client:family`、`client:command`、`client:volunteer`、`area:backend`、`ai-human-review`、`privacy`、`demo`、`testing`。
- **内容**：先实现确定性摘要，分为最后确认信息、已确认线索、待核实事项、已排除方向、当前任务状态和安全提示。不同角色返回不同摘要视图。
- **验收标准**：
  - 未核实线索绝不被写成确认事实；摘要有生成时间和来源范围。
  - 家属不见内部任务/搜索方向；志愿者只见本人任务必要信息；指挥见完整内部摘要。
  - 无外部 AI 也可工作；接口不因模型超时阻断案件操作。

### G-02 `[API] POST /api/cases/{case_id}/summary-drafts - 生成待审核 AI 案情草稿`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=1.5d`，`Demo Step=指挥研判`。
- **标签**：`type:api`、`module:ai`、`module:case`、`client:command`、`area:backend`、`area:integration`、`ai-human-review`、`security`、`demo`、`testing`。
- **内容**：通过通义千问/百炼适配层，在受控输入范围内生成案情草稿；保存模板版本、模型标识、数据范围、草稿和审核状态。
- **验收标准**：草稿标记 `draft`，不能自动覆盖已发布摘要；模型失败时返回可解释状态并保留确定性摘要；提示注入文本不被当作系统指令；不向模型发送无必要的联系方式、完整病史或精确轨迹。

### G-03 `[API] PATCH /api/cases/{case_id}/summary-drafts/{draft_id}/review - 审核并发布案情草稿`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 8-11`，`Estimate=1d`，`Demo Step=指挥研判`。
- **标签**：`type:api`、`module:ai`、`module:case`、`client:command`、`area:backend`、`ai-human-review`、`security`、`testing`。
- **内容**：指挥对 AI 草稿执行编辑、批准或拒绝；批准后生成版本化摘要，保留审核人、时间、原草稿与最终文本引用。
- **验收标准**：仅案件 commander 可审核；不能发布含未确认事实/越权字段的文本；拒绝不会改变当前已发布摘要；所有操作有审计记录。

### G-04 `[Feature] 指挥端 - 公共实时案情板与 AI 草稿审核`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 8-11`，`Estimate=1d`，`Demo Step=指挥研判`。
- **标签**：`type:feature`、`module:ai`、`module:case`、`client:command`、`area:frontend`、`ai-human-review`、`demo`。
- **内容**：指挥页清楚区分“已确认”“待核实”“已排除”“任务状态”“AI 草稿/人工已发布”，支持一键刷新确定性摘要和 P1 草稿审核。
- **验收标准**：页面不以 AI 输出替代人工事实；每个摘要显示更新时间和审核状态；线索审核或任务状态变化后刷新可见；演示可在无模型密钥情况下走完整替代路径。

---

# H. 学习中心、知识问答与案例经验库模块

## H1. 学习中心子模块

### H-01 `[API] GET /api/learning/resources - 获取队伍介绍、手册与防走失知识`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 12-15`，`Estimate=0.5d`，`Demo Step=学习中心`。
- **标签**：`type:api`、`module:knowledge`、`client:family`、`client:volunteer`、`client:learner`、`area:backend`、`privacy`、`testing`。
- **内容**：提供可公开/协作级别的队伍介绍、搜救手册、防走失知识和任务前注意事项。内容以版本化静态/数据库资源管理。
- **验收标准**：资源有标题、来源、版本、生效时间和可见级别；新人无案件权限也能访问学习资源；不包含真实案件隐私或内部原始聊天。

### H-02 `[API] GET /api/learning/questions - 获取理论题库`

- **字段**：`Priority=P2 - 后续规划`，`Sprint=Post-MVP`，`Estimate=1d`，`Demo Step=学习中心`。
- **标签**：`type:api`、`module:knowledge`、`client:learner`、`area:backend`、`testing`。
- **内容**：返回选择题、判断题、情景题和案例题，支持按标签/难度筛选；正确答案不在题目列表中直接泄露。
- **验收标准**：题目与答案分离；题目来源和适用范围可追溯；未登录/新人/志愿者的访问范围按策略确定。

### H-03 `[API] POST /api/learning/questions/{question_id}/answers - 提交题目答案并获取解析`

- **字段**：`Priority=P2 - 后续规划`，`Sprint=Post-MVP`，`Estimate=0.5d`，`Demo Step=学习中心`。
- **标签**：`type:api`、`module:knowledge`、`client:learner`、`area:backend`、`testing`。
- **内容**：接收答题结果，返回正确性和带来源的解释；后续可记录学习进度，但 MVP 不以分数作为任务权限依据。
- **验收标准**：答案校验在服务端；解析带知识资源来源；错误输入不可获得其他题正确答案或修改题库。

### H-04 `[API] POST /api/knowledge/ask - 基于已审核知识资源问答`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 12-15`，`Estimate=1.5d`，`Demo Step=学习中心`。
- **标签**：`type:api`、`module:knowledge`、`module:ai`、`client:learner`、`client:volunteer`、`area:backend`、`area:integration`、`ai-human-review`、`privacy`、`demo`、`testing`。
- **内容**：仅基于已审核手册、脱敏案例和防走失知识进行 RAG/规则问答，例如“老人走失第一小时应该做什么”。
- **验收标准**：回答必须携带来源；没有可靠来源时明确回答“不确定/请联系负责人”；不检索未脱敏案例、完整病史、原始聊天或精确轨迹；提示注入与无来源回答有测试。

### H-05 `[Feature] 学习中心 - 手册、案例和新人问答入口`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 12-15`，`Estimate=1d`，`Demo Step=学习中心`。
- **标签**：`type:feature`、`module:knowledge`、`client:learner`、`client:volunteer`、`area:frontend`、`demo`、`testing`。
- **内容**：新增新人/志愿者学习首页，展示手册、防走失知识、案例卡片和问答入口；与案件操作区域隔离。
- **验收标准**：新人账号可访问学习页但不能访问案件操作页面；问答显示来源和非确定性提示；无 AI 时仍可浏览静态手册。

## H2. 案例归档与经验库子模块

### H-06 `[API] POST /api/cases/{case_id}/archive-drafts - 创建案件归档与复盘草稿`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 12-15`，`Estimate=1d`，`Demo Step=案例归档`。
- **标签**：`type:api`、`module:archive`、`module:ai`、`client:command`、`client:admin`、`area:backend`、`ai-human-review`、`privacy`、`testing`。
- **内容**：指挥/授权后台从已结束案件的确认线索、任务结果、简化轨迹和复盘文本创建归档草稿；禁止将原始敏感材料直接公开。
- **验收标准**：只有 `resolved/closed` 案件可归档；草稿与正式案例分离；输入范围和脱敏状态可追溯；不自动发布到知识库。

### H-07 `[API] POST /api/archive-drafts/{draft_id}/deidentify - 执行或确认脱敏`

- **字段**：`Priority=P2 - 后续规划`，`Sprint=Post-MVP`，`Estimate=1.5d`，`Demo Step=案例归档`。
- **标签**：`type:api`、`module:archive`、`client:admin`、`area:backend`、`ai-human-review`、`privacy`、`security`、`testing`。
- **内容**：对案例草稿识别并替换姓名、联系方式、精确住址、完整病史、原始聊天和精确轨迹；人工审核脱敏结果。
- **验收标准**：脱敏前后版本可比较；无法可靠脱敏时禁止发布；管理员操作保留审计；不承诺对任意文本自动完全脱敏。

### H-08 `[API] PATCH /api/archive-drafts/{draft_id}/review - 审核并发布经验案例`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 12-15`，`Estimate=1d`，`Demo Step=案例归档`。
- **标签**：`type:api`、`module:archive`、`client:admin`、`area:backend`、`ai-human-review`、`privacy`、`testing`。
- **内容**：授权人员编辑、批准或拒绝案例草稿；批准后写入经验标签、时间线、复盘要点和适用范围。
- **验收标准**：未审核草稿不可出现在学习中心/RAG；发布记录审核人、版本与来源；拒绝不影响原始案件和已发布案例。

### H-09 `[Feature] 管理端 - 模拟聊天导入、复盘草稿与案例发布`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 12-15`，`Estimate=1.5d`，`Demo Step=案例归档`。
- **标签**：`type:feature`、`module:archive`、`module:ai`、`client:admin`、`area:frontend`、`ai-human-review`、`privacy`、`demo`。
- **内容**：为演示使用虚构群聊记录创建导入、结构化摘要、脱敏确认、案例生成与发布页面；可为初赛做受控模拟，不接收真实群聊。
- **验收标准**：界面持续显示“模拟/脱敏数据”；未审核内容不可发布；可展示“聊天记录 → 线索 → 时间线/经验点”的变化；失败时可用预置草稿完成演示。

---

# I. 管理端、审计与运维模块

### I-01 `[API] GET /api/admin/audit-events - 查询审计事件`

- **字段**：`Priority=P1 - 演示增强`，`Sprint=Day 12-15`，`Estimate=1d`，`Demo Step=基础设施`。
- **标签**：`type:api`、`module:admin`、`area:backend`、`security`、`privacy`、`testing`。
- **内容**：授权管理员按案件、资源类型、事件类型和时间范围查询审计事件；默认不返回敏感详情 JSON。
- **验收标准**：访问本身被审计；分页与时间范围受限；响应不含密码、token、完整病史、原始聊天、精确位置和未处理附件内容。

### I-02 `[Feature] 管理端 - 账号、审计与演示系统状态`

- **字段**：`Priority=P2 - 后续规划`，`Sprint=Post-MVP`，`Estimate=1.5d`，`Demo Step=基础设施`。
- **标签**：`type:feature`、`module:admin`、`client:admin`、`area:frontend`、`security`、`privacy`、`testing`。
- **内容**：管理端提供账号状态、API 健康、审计查询和演示数据状态；不把管理端变成可直接查看所有案件明文的后门。
- **验收标准**：管理员无案件成员关系时仍不能从业务页面读取案件详情；敏感操作需明确确认；页面不显示凭据、密钥和完整敏感日志。

### I-03 `[Quality] OpenAPI、前后端类型与权限矩阵对齐`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 12-15`，`Estimate=1d`，`Demo Step=基础设施`。
- **标签**：`type:quality`、`area:backend`、`area:frontend`、`area:docs`、`security`、`testing`。
- **内容**：每新增端点同步更新 `docs/openapi.yaml`、`docs/API.md`、前端 API 类型、错误码、角色权限表与演示说明。
- **验收标准**：无实现但文档宣称已完成的能力；请求/响应字段与前端类型一致；每个端点在 OpenAPI 中明确认证、角色、错误码和隐私裁剪。

---

# J. 测试、演示与文章交付模块

### J-01 `[Data] 演示账号与模拟案件 Fixture - 一键初始化和重置`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 12-15`，`Estimate=1d`，`Demo Step=基础设施`。
- **标签**：`type:data`、`area:backend`、`area:database`、`demo`、`privacy`、`testing`。
- **内容**：扩展现有 bootstrap 命令，建立家属、指挥、志愿者、新人账号及一例虚构案件、线索、地点、两项任务、模拟位置和预置归档草稿；提供可重复重置命令。
- **验收标准**：迁移后可按文档启动；重复运行不产生重复数据；仓库不含真实密码、照片、联系方式和真实坐标；不需要手工 SQL 才能走通 Demo。

### J-02 `[Quality] 三端核心闭环 E2E 验证 - 家属到指挥到志愿者`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 12-15`，`Estimate=1d`，`Demo Step=基础设施`。
- **标签**：`type:quality`、`client:family`、`client:command`、`client:volunteer`、`area:backend`、`area:frontend`、`security`、`privacy`、`demo`、`testing`。
- **内容**：自动化/手工脚本验证家属问询确认建案、邀请指挥、指挥审核/派单、志愿者执行/反馈、指挥确认、地图/摘要刷新全过程。
- **验收标准**：全过程无需数据库手工修改；每步存在可观察 UI 和数据状态变化；至少覆盖一次非成员越权、志愿者越权查看、AI/地图服务失败降级；测试输出无敏感数据。

### J-03 `[Delivery] 初赛 Demo 脚本、录屏与离线降级包`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 12-15`，`Estimate=1d`，`Demo Step=基础设施`。
- **标签**：`type:delivery`、`area:docs`、`demo`、`privacy`、`testing`。
- **内容**：制作 3-5 分钟固定讲解脚本、分镜、账号切换顺序、预期界面变化、地图/AI 失败时的静态地图和预录屏兜底。
- **验收标准**：故事线覆盖 AI 问询、人工审核、地图、任务、位置回传、案例沉淀和安全边界；所有画面使用模拟数据；不声称人脸识别、真实监控接入、自动派遣或预测老人位置。

### J-04 `[Delivery] 技术文章与路演素材 - 从 Issue 证据整理交付`

- **字段**：`Priority=P0 - 初赛必需`，`Sprint=Day 12-15`，`Estimate=1d`，`Demo Step=基础设施`。
- **标签**：`type:delivery`、`area:docs`、`demo`、`privacy`。
- **内容**：基于已关闭 Issue、测试结果、架构图和截图写项目文章，并制作 10 页以内路演 PPT 的技术素材。
- **验收标准**：文章能说明问题、三端闭环、状态机、权限边界、AI 人工审核与地图降级；每项“已完成”都有代码/测试/Demo 证据；不夸大为真实搜救生产系统或 AI 定位能力。

---

## 3. 初赛推荐创建顺序（避免 Issue 一次全部进入进行中）

### 3.0 GitHub 批量创建与编号回传规则

- 以**批次**为单位创建：同一批次中的 Issue 通过一次批量脚本连续创建，不逐条要求
  人工操作。
- 每个 Issue 创建时设置 GitHub 原生 `Type`、`Priority` 与本草案规定的官方 Label；
  仅使用本节定义的 `backend` / `frontend` 端别 Label，不创建其他自定义 Label 或自定义 Field。
- 每个批次结束后，记录并回传完整映射：`草案编号 -> GitHub #Issue 编号 -> 标题`。
  后续开发、关闭和讨论都以 GitHub `#编号` 为准。
- GitHub 全仓库 Issue 编号是连续的；若创建期间有其他人新建 Issue，编号可能有间隔，
  映射表而不是“连续编号假设”是唯一准确依据。
- 每个新建 Issue 默认保持 Open。只有通过正文全部验收标准、附有测试/验证证据后才能
  关闭。`B-00` 是首个质量门禁，不得在创建后直接关闭。

### 批次 0：现有 API 质量门禁

`B-00`。创建后立即进入 `Ready`，先补齐 11 个端点的独立测试文件和 OpenAPI 对齐。

### 批次 1：账号基线与家属建档

`A-01`、`B-01`、`B-02`、`B-03`、`B-04`、`B-09`、`C-03`。

### 批次 2：案件、线索与指挥研判

`C-01`、`C-02`、`C-05`、`D-01`、`D-02`、`D-03`、`D-06`。

### 批次 3：任务、位置与志愿者执行闭环

`E-01`、`E-02`、`E-03`、`E-04`、`E-05`、`E-06`、`E-09`、`E-10`、`E-11`。

### 批次 4：地图、公共案情与初赛演示增强

`B-06`、`B-07`、`F-01`、`F-05`、`G-01`、`G-04`。

### 批次 5：P1 功能与案例沉淀

所有余下 P1 Issue：`A-04`、`A-05`、`B-05`、`B-08`、`B-10`、`C-04`、`D-04`、
`D-05`、`E-07`、`E-08`、`F-02`、`F-03`、`F-06`、`G-02`、`G-03`、`H-01`、
`H-04`、`H-05`、`H-06`、`H-08`、`H-09`、`I-01`。

### 批次 6：P2 路线图

所有 P2 Issue 只创建为 Open Backlog，不进入本次初赛开发队列。

### 第一批：Day 1-3，家属建档最小闭环

`A-01`、`B-01`、`B-02`、`B-03`、`B-04`、`B-09`、`C-03`。

### 第二批：Day 4-7，指挥线索与志愿者任务闭环

`C-01`、`C-02`、`C-05`、`D-01`、`D-02`、`D-03`、`D-06`、`E-01`、`E-02`、`E-03`、`E-04`、`E-05`、`E-06`、`E-10`、`E-11`。

### 第三批：Day 8-11，地图、摘要和演示增强

`B-06`、`B-07`、`F-01`、`F-05`、`G-01`、`G-04`。P0 稳定后再择机加入 `D-04`、`F-02`、`F-03`、`F-06`、`G-02`、`G-03`。

### 第四批：Day 12-15，固定数据、验收、归档和交付

`E-09`、`H-01`、`H-04`、`H-05`、`H-06`、`H-08`、`H-09`、`I-01`、`I-03`、`J-01`、`J-02`、`J-03`、`J-04`。

### Post-MVP：只创建 Backlog，不排入本次 15 天

`A-02`、`A-03`、`A-06`、`A-07`、`B-08`、`E-07`、`E-08`、`F-04`、`H-02`、`H-03`、`H-07`、`I-02`，以及所有 P2 Issue。

## 4. 发布前需要你确认的产品决策

1. 家属提交问询后，是“家属确认即自动创建案件”，还是“指挥确认后才正式建案”？草案当前采用前者。
2. 初赛任务是否固定为“一项任务只分配一名志愿者”？草案当前采用单受领人，避免任务协同复杂度。
3. 初赛是否真的接入高德真实底图/POI，还是先用静态模拟地图并保留高德适配接口？
4. 初赛是否实际调用通义千问/百炼，还是以规则化草稿 + 预置 AI 示例作为可离线 Demo？
5. 附件上传、密码找回、账号管理是否全部保持 P2，不进入本次 15 天核心范围？
6. 是否建立 GitHub Project，并采用本草案的 `Status/Priority/Sprint/Estimate/Demo Step` 五个字段？

---

## 5. GitHub Issue 编号映射（已发布）

所有 Issue 均保持 Open。`#2` 是现有 API 的质量门禁，**只有 11 个端点均有独立测试、
OpenAPI 对齐且测试命令全量通过后才能关闭**。

| 发布批次 | 草案编号 → GitHub Issue 编号 |
| --- | --- |
| 批次 0：质量门禁 | `B-00 → #2` |
| 批次 1：账号与家属建档 | `A-01 → #3`、`B-01 → #4`、`B-02 → #5`、`B-03 → #6`、`B-04 → #7`、`B-09 → #8`、`C-03 → #9` |
| 批次 2：案件、线索与指挥研判 | `C-01 → #10`、`C-02 → #11`、`C-05 → #12`、`D-01 → #13`、`D-02 → #14`、`D-03 → #15`、`D-06 → #16` |
| 批次 3：任务与志愿者执行 | `E-01 → #17`、`E-02 → #18`、`E-03 → #19`、`E-04 → #20`、`E-05 → #21`、`E-06 → #22`、`E-09 → #23`、`E-10 → #24`、`E-11 → #25` |
| 批次 4：地图与公共案情 | `B-06 → #26`、`B-07 → #27`、`F-01 → #28`、`F-05 → #29`、`G-01 → #30`、`G-04 → #31` |
| 批次 5：P1 增强与案例沉淀 | `A-04 → #32`、`A-05 → #33`、`B-05 → #34`、`B-08 → #35`、`B-10 → #36`、`C-04 → #37`、`D-04 → #38`、`D-05 → #39`、`E-07 → #40`、`E-08 → #41`、`F-02 → #42`、`F-03 → #43`、`F-06 → #44`、`G-02 → #45`、`G-03 → #46`、`H-01 → #47`、`H-04 → #48`、`H-05 → #49`、`H-06 → #50`、`H-08 → #51`、`H-09 → #52`、`I-01 → #53` |
| P0：契约与交付补充 | `I-03 → #54`、`J-01 → #55`、`J-02 → #56`、`J-03 → #57`、`J-04 → #58` |
| 批次 6：P2 路线图 Backlog | `A-02 → #59`、`A-03 → #60`、`A-06 → #61`、`A-07 → #62`、`F-04 → #63`、`H-02 → #64`、`H-03 → #65`、`H-07 → #66`、`I-02 → #67` |

### 发布属性复核

- 共发布 66 条计划 Issue：`#2` 至 `#67`；仓库既有关闭 Bug 为 `#1`。
- P0 草案事项已设置原生 `Priority=High`；P1 为 `Medium`；P2 为 `Low`。
- 新增 API/功能/数据能力使用 `Type=Feature` 或 `Task` 与官方 `enhancement` 标签；
  文档交付使用官方 `documentation` 标签；质量验收任务默认不打功能标签；`#2`–`#67` 另统一补加 `backend`。
- 已逐条核对 66 条 GitHub 标题、Type 与官方 Label；`backend` 的批量补加以本节为准。

---

## 6. AI 调用可靠性与合规路由补充 Issue（已发布）

> 本组事项是 `B-02`、`D-04`、`G-02`、`H-04` 等实际模型调用能力的共同前置条件。当前
> `rule_based` 问询和人工处理路径仍是必需的最终降级，而不是由多个模型供应商替代。
>
> 多线路不得理解为把同一份案件数据盲目发送到更多供应商。只有在数据分类、用户授权、
> 数据驻留区域、供应商协议和功能用途均允许时，才可以切换到备用线路。任何线路均不得让
> AI 草稿自动成为已确认线索、公开案情或现场任务。

### AI-R-01 `[Architecture] AI Gateway - Provider 抽象、能力注册与安全配置`

- **字段**：`Priority=P0 - 初赛前置`，`Sprint=基础设施`，`Estimate=2d`，`Demo Step=AI 基础设施`。
- **标签**：`type:architecture`、`area:backend`、`ai`、`privacy`、`reliability`。
- **关联**：前置 `B-02`、`D-04`、`G-02`、`H-04`；依赖 `I-03` 的配置与契约校验。
- **内容**：定义与供应商 SDK 解耦的 AI Gateway 和 `AiProvider` 能力接口，按问询、结构化
  提取、案情摘要、知识问答、案例整理分别注册能力。Provider 配置包含供应商/区域/模型、
  可处理数据等级、允许用途、输入输出限制、是否允许作为备用线路、优先级、权重和紧急停用
  状态；密钥仅来自运行环境或受控密钥系统。
- **验收标准**：
  - 业务服务不得直接依赖单一供应商 SDK 或在路由中拼接模型请求；无有效 Provider 配置时
    安全返回确定性降级状态。
  - 选择 Provider 前同时校验请求能力、数据等级、用途、数据区域和线路启用状态；不满足
    任一条件的线路不得进入候选集。
  - 配置示例不含密钥、生产 URL 或真实案件数据；启动时拒绝无模型标识、无超时、无数据策略
    或“允许敏感数据但未声明合规范围”的危险配置。
  - 每次执行保存 Provider/模型/模板版本、输入范围引用或哈希、脱敏策略版本和任务状态，
    但普通日志与审计元数据不复制原始聊天、完整病史、联系方式或精确轨迹。

### AI-R-02 `[Reliability] AI 准入控制 - 分功能队列、并发上限、速率与预算`

- **字段**：`Priority=P0 - 初赛前置`，`Sprint=基础设施`，`Estimate=2d`，`Demo Step=AI 基础设施`。
- **标签**：`type:reliability`、`area:backend`、`ai`、`security`。
- **关联**：依赖 `AI-R-01`；前置 `D-04`、`G-02`、`H-04` 的真实模型调用。
- **内容**：为实时问询、线索提取、摘要、知识问答和批量归档设置独立准入池；同时限制全局、
  功能、用户/案件和 Provider 的并发、请求速率、token 预算与成本预算。耗时任务使用可恢复
  的异步 Job 状态机和幂等键，避免网络重试、重复点击或 worker 重启产生重复模型调用。
- **验收标准**：
  - 批量归档或摘要任务不能耗尽实时问询容量；每个池的排队超时、最大并发和拒绝响应可配置
    且有测试。
  - Provider 的并发、RPM/TPM、日预算和 `Retry-After` 均被执行；达到限制时不自旋重试，
    返回可解释的 `queued`、`rate_limited`、`budget_exhausted` 或确定性降级状态。
  - 具有副作用或成本的 AI Job 使用服务端幂等键；同一键在并发提交、客户端重试和 worker
    重启后最多执行一次，或返回同一既有任务及结果。
  - 生产多实例部署不只依赖进程内计数；共享限流或持久化队列的实现、故障恢复边界和本地
    单实例降级行为均有文档与自动化测试。

### AI-R-03 `[Reliability] AI 执行器 - Deadline、可恢复重试、退避与熔断`

- **字段**：`Priority=P0 - 初赛前置`，`Sprint=基础设施`，`Estimate=2d`，`Demo Step=AI 故障演示`。
- **标签**：`type:reliability`、`area:backend`、`ai`、`testing`。
- **关联**：依赖 `AI-R-01`、`AI-R-02`；前置所有外部模型调用。
- **内容**：实现统一执行器，设置排队、连接、首字节和总响应的绝对 deadline；按错误类别
  执行有限重试、指数退避与随机抖动，并为每条 Provider 线路维护独立的熔断器和半开探测。
  结构化输出校验失败可做一次“只修复格式、不得新增事实”的受控请求，随后进入人工处理。
- **验收标准**：
  - 只对连接瞬断、超时、`408`、明确可恢复的 `429` 与短暂 `5xx` 重试；`400/401/403`、
    内容安全拒绝、请求校验失败和连续结构化输出错误不得重试。
  - 总 deadline 覆盖排队与所有尝试，任何请求不会无限等待；`429` 优先遵守 `Retry-After`。
  - 熔断器具有 `closed`、`open`、`half_open` 状态；连续故障后停止向故障线路发送常规请求，
    冷却后仅允许受控探测，并在恢复后逐步放量。
  - 模拟连接失败、超时、429、5xx、畸形 JSON 和连续失败，验证重试次数、等待上限、熔断
    状态和最终用户可见降级结果；测试不得输出敏感输入原文。

### AI-R-04 `[Reliability] 合规 Failover 与确定性降级策略`

- **字段**：`Priority=P0 - 初赛前置`，`Sprint=基础设施`，`Estimate=1.5d`，`Demo Step=AI 故障演示`。
- **标签**：`type:reliability`、`area:backend`、`ai`、`privacy`、`security`。
- **关联**：依赖 `AI-R-01`、`AI-R-03`；落实 `B-01`、`D-04`、`G-02`、`H-04` 的服务失败降级。
- **内容**：定义可审计的四级策略：本线路有限重试、同 Provider 的同等合规备用端点/模型、
  已批准的替代 Provider、规则化或人工处理。每个能力独立配置是否允许自动切换；默认禁止
  将更高敏感等级数据跨 Provider 转发。
- **验收标准**：
  - 备用线路仅可从 `AI-R-01` 校验后的合规候选集中选择；失败切换不得扩大字段范围、
    改变数据区域、绕过用户授权或发送完整病史/联系方式/精确轨迹。
  - 无合规备用线路、预算耗尽、熔断开启或总 deadline 到达时，立即进入规则化/人工路径，
    保持案件创建、线索提交和已发布摘要可继续使用。
  - API 与 UI 清楚区分 `provider_fallback`、`rule_based`、`manual_required` 与 `failed`；
    不将最终降级伪装成 AI 成功结果。
  - 每次路由、切换和降级记录无敏感内容的原因码、候选线路、实际线路和结果状态；配置
    禁止时不会向备用供应商发送请求的测试必须通过。

### AI-R-05 `[Security] AI 出站数据最小化、脱敏与 Provider 数据策略`

- **字段**：`Priority=P0 - 初赛前置`，`Sprint=安全与隐私`，`Estimate=2d`，`Demo Step=安全边界`。
- **标签**：`type:security`、`area:backend`、`ai`、`privacy`。
- **关联**：依赖 `AI-R-01`；前置 `D-04`、`G-02`、`H-04`；关联 `docs/SECURITY_AND_PRIVACY.md`。
- **内容**：为每种 AI 能力定义允许字段、脱敏规则、最大上下文、保留期限和 Provider 数据处理
  条款；在 AI Gateway 前执行字段级裁剪和提示注入隔离。将请求数据分类和供应商可处理范围
  固化为可审查策略，而不是依赖调用者约定。
- **验收标准**：
  - 摘要、问询、提取和 RAG 各自有最小输入白名单；完整联系方式、完整病史、精确轨迹、
    原始聊天和无权限字段默认不出站。
  - 文档和用户输入均被视为不可信数据，不能改写系统权限、工具调用规则或跨越检索范围；
    提示注入、越权提问和无来源回答均有自动化测试。
  - 日志、指标标签、错误报告和死信记录只保存必要的 ID、哈希、版本和原因码；可审计地
    关联任务但不能从普通运维日志恢复敏感原文。
  - Provider 策略明确区域、保留/训练约束、数据等级和紧急停用条件；策略变更需审查，
    不满足策略的请求被拒绝或走人工降级。

### AI-R-06 `[Observability] AI 调用指标、成本、链路追踪与告警`

- **字段**：`Priority=P1 - 生产准备`，`Sprint=可观测性`，`Estimate=1.5d`，`Demo Step=运行状态`。
- **标签**：`type:observability`、`area:backend`、`ai`、`reliability`。
- **关联**：依赖 `AI-R-01` 至 `AI-R-04`；关联 `J-02`、`J-03`。
- **内容**：提供按能力、Provider、模型和结果状态聚合的指标、结构化链路追踪与告警规则，
  用于发现排队、超时、429、熔断、降级率、token 使用和预算异常；不可将案件、用户或
  原始提示词作为高基数指标标签。
- **验收标准**：
  - 可查看成功率、p50/p95/p99 延迟、排队时间、重试次数、429/5xx、熔断状态、各级
    降级率、token/成本和预算余额；指标按能力与 Provider 可过滤。
  - 任务可在不暴露敏感正文的前提下从 API 请求关联到 AI Job、Provider 路由和草稿结果。
  - 为持续失败、异常降级率、预算接近耗尽和队列积压设置告警；告警包含处置链接或运行
    手册引用，而不含敏感请求内容。
  - 模拟故障时仪表盘与告警显示正确，且指标基数、采样和保留策略不会造成可用性或隐私风险。

### AI-R-07 `[Quality] AI Provider Mock、故障注入与降级矩阵测试`

- **字段**：`Priority=P0 - 初赛前置`，`Sprint=质量验收`，`Estimate=2d`，`Demo Step=AI 故障演示`。
- **标签**：`type:quality`、`area:backend`、`ai`、`testing`、`privacy`。
- **关联**：依赖 `AI-R-01` 至 `AI-R-05`；补充 `J-02` 的 AI 失败 E2E 验收。
- **内容**：构建可脚本化的 Provider Mock 与故障注入矩阵，验证单线路恢复、合规 Provider
  切换、无候选时确定性降级、并发隔离、幂等、熔断恢复、敏感数据阻断和 UI/API 状态呈现。
- **验收标准**：
  - 覆盖 DNS/连接失败、超时、408、429、5xx、畸形 JSON、内容安全拒绝、Provider 熔断、
    半开探测、预算耗尽和队列积压；每种情形有确定的期望路由和用户可见结果。
  - 覆盖“备用 Provider 被数据政策禁止”和“允许切换但字段范围不变”两种场景，证明失败
    不会扩大数据出境或泄露无权限字段。
  - 覆盖同一 AI Job 的并发提交、客户端重试和 worker 重启；草稿、审计和成本记录不重复。
  - 端到端测试至少验证一次模型失败后家属仍可完成规则问询、指挥仍可编辑确定性摘要、
    线索仍可进入人工审核；测试夹具和输出均使用虚构、脱敏数据。

### AI-R-08 `[Operations] AI 灰度发布、Kill Switch 与故障处置手册`

- **字段**：`Priority=P1 - 生产准备`，`Sprint=运行准备`，`Estimate=1d`，`Demo Step=故障处置`。
- **标签**：`type:operations`、`area:docs`、`area:backend`、`ai`、`reliability`。
- **关联**：依赖 `AI-R-01` 至 `AI-R-06`；关联 `J-03`、`J-04`。
- **内容**：定义能力级和 Provider 级灰度、权重调整、紧急停止、配置回滚、预算熔断、
  故障演练和人工接管流程；记录谁可执行、触发条件、审计要求和恢复验证。
- **验收标准**：
  - 可以不重启或在受控重启后按能力、Provider 或模型紧急停用，并立即让新请求走规则化
    或人工路径；已有草稿和已发布内容不被删除或覆盖。
  - 灰度配置包含受众、比例、有效期、回滚条件和负责人；不得只根据模型置信度自动扩大流量。
  - 运行手册包含超时/429/预算耗尽/隐私策略拒绝/供应商故障的诊断、止损、人工沟通和
    恢复步骤，并引用可观测性面板。
  - 至少完成一次使用模拟 Provider 的故障演练，保留脱敏的演练记录和改进事项。

### 发布编号映射

| 草案编号 | GitHub Issue | 标题 |
| --- | --- | --- |
| `AI-R-01` | [#96](https://github.com/AG-Angui/Angui/issues/96) | AI Gateway - Provider 抽象、能力注册与安全配置 |
| `AI-R-02` | [#97](https://github.com/AG-Angui/Angui/issues/97) | AI 准入控制 - 分功能队列、并发上限、速率与预算 |
| `AI-R-03` | [#98](https://github.com/AG-Angui/Angui/issues/98) | AI 执行器 - Deadline、可恢复重试、退避与熔断 |
| `AI-R-04` | [#99](https://github.com/AG-Angui/Angui/issues/99) | 合规 Failover 与确定性降级策略 |
| `AI-R-05` | [#100](https://github.com/AG-Angui/Angui/issues/100) | AI 出站数据最小化、脱敏与 Provider 数据策略 |
| `AI-R-06` | [#101](https://github.com/AG-Angui/Angui/issues/101) | AI 调用指标、成本、链路追踪与告警 |
| `AI-R-07` | [#102](https://github.com/AG-Angui/Angui/issues/102) | AI Provider Mock、故障注入与降级矩阵测试 |
| `AI-R-08` | [#103](https://github.com/AG-Angui/Angui/issues/103) | AI 灰度发布、Kill Switch 与故障处置手册 |
