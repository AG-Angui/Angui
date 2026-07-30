# API 说明

## 1. 当前范围

当前 API 提供用于 MVP 开发的认证、案件成员授权、案件和线索纵向闭环。它已经连接 SeaORM 数据层，并实现可撤销数据库会话和服务端字段裁剪；密码找回、MFA、账号审批、组织模型和生产级安全运营仍未实现。请求只允许使用虚构或充分脱敏的数据。

默认地址为 `http://127.0.0.1:8080`，所有业务接口位于 `/api` 下。

机器可读的接口契约见 [openapi.yaml](./openapi.yaml)。它与本说明覆盖同一组已注册端点；其中每个 operation 都显式列出认证要求、全局/案件角色限制、错误响应和字段可见性。以服务端路由和测试为准，文档不得把尚未注册的规划接口描述为已实现。

## 2. 接口列表

| 方法 | 路径 | 成功状态 | 用途 |
| --- | --- | --- | --- |
| `GET` | `/api/health` | `200` | 服务健康检查 |
| `POST` | `/api/auth/login` | `200` | 使用邮箱和密码创建短期会话 |
| `GET` | `/api/auth/me` | `200` | 获取当前认证用户 |
| `POST` | `/api/auth/logout` | `204` | 撤销当前服务端会话 |
| `POST` | `/api/intake-sessions` | `201` | 创建成员的未确认走失信息问询会话 |
| `POST` | `/api/intake-sessions/{session_id}/answers` | `201` | 追加一个未确认答案并获取下一问 |
| `GET` | `/api/intake-sessions/{session_id}/profile-draft` | `200` | 获取家属专属、待确认的标准化画像草稿 |
| `POST` | `/api/intake-sessions/{session_id}/confirm` | `201` | 家属确认画像并创建正式案件 |
| `GET` | `/api/cases` | `200` | 按创建时间倒序列出案件 |
| `GET` | `/api/cases/command-intake` | `200` | 指挥查看仅含最小接案信息的待受理队列 |
| `POST` | `/api/cases` | `201` | 创建案件和老人画像 |
| `GET` | `/api/cases/{case_id}` | `200` | 查询案件、老人画像和线索 |
| `PATCH` | `/api/cases/{case_id}/status` | `200` | 人工更新案件状态 |
| `POST` | `/api/cases/{case_id}/accept-command` | `200` | 指挥人工受理待受理案件并获得该案指挥权限 |
| `GET` | `/api/cases/{case_id}/clues` | `200` | 获取角色裁剪、可分页的线索时间轴 |
| `POST` | `/api/cases/{case_id}/clues` | `201` | 提交待审核线索 |
| `GET` | `/api/cases/{case_id}/map-view` | `200` | 获取含文字地点回退的角色裁剪地图态势 |
| `GET` | `/api/cases/{case_id}/summary` | `200` | 获取含来源范围与生成时间的角色裁剪确定性案件摘要 |
| `GET` | `/api/cases/{case_id}/public-progress` | `200` | 家属查看仅含已确认信息和本人待补充事项的公开进展 |
| `POST` | `/api/cases/{case_id}/clue-drafts` | `201` | 将受控文本持久化为不可直接确认的线索草稿 |
| `GET` | `/api/cases/{case_id}/pois` | `200` | 按服务端授权中心查询有上限的周边资源，失败时明确降级 |
| `POST` | `/api/cases/{case_id}/summary-drafts` | `201` | 指挥创建带来源范围和版本的内部摘要草稿 |
| `PATCH` | `/api/cases/{case_id}/summary-drafts/{draft_id}/review` | `200` | 指挥提交、审核发布、驳回或撤回摘要草稿 |
| `POST` | `/api/cases/{case_id}/archive-drafts` | `201` | 指挥为已结束案件创建受控内部归档草稿 |
| `GET` | `/api/cases/{case_id}/places` | `200` | 获取按案件角色、审核状态和可见级别裁剪的地点 |
| `POST` | `/api/cases/{case_id}/places` | `201` | 家属或指挥提交常去/关键地点，始终待人工审核 |
| `GET` | `/api/cases/{case_id}/resource-configuration` | `200` | 获取当前案件可用的地点类型和图片限制 |
| `POST` | `/api/cases/{case_id}/attachments` | `201` | 上传受控 JPEG/PNG 案件图片，始终待人工审核 |
| `GET` | `/api/cases/{case_id}/attachments/{attachment_id}` | `200` | 按案件权限下载本人上传的图片，指挥可下载案件全部图片 |
| `POST` | `/api/clues/{clue_id}/attachments` | `201` | 为具体线索上传待审核 JPEG/PNG 佐证图片 |
| `POST` | `/api/cases/{case_id}/members` | `201` | 家属邀请指挥，或指挥添加案件成员 |
| `GET` | `/api/cases/{case_id}/members` | `200` | 指挥查看本案件成员，用于协作审查和人工选择已授权志愿者 |
| `PATCH` | `/api/clues/{clue_id}/review` | `200` | 人工审核线索 |
| `GET` | `/api/admin/audit-events` | `200` | 管理员分页查看经过脱敏的审计事件 |
| `GET` | `/api/admin/users` | `200` | 管理员分页查看不含凭据的账号状态 |
| `PATCH` | `/api/admin/users/{user_id}/status` | `200` | 管理员启用、停用或锁定账号 |
| `POST` | `/api/admin/archive-drafts/{draft_id}/deidentify` | `200` | 管理员记录归档草稿的人工脱敏确认或拒绝 |
| `PATCH` | `/api/admin/archive-drafts/{draft_id}/review` | `200` | 管理员发布、拒绝或撤回已脱敏归档草稿 |

除健康检查和登录外，所有接口都要求：

```http
Authorization: Bearer angui_<session-token>
```

令牌原文只在登录响应中返回一次，数据库仅保存 SHA-256 哈希。前端当前把令牌保存在标签页级 `sessionStorage`；正式部署必须使用 HTTPS。

## 3. 认证响应

登录请求：

```json
{
  "email": "family@demo.invalid",
  "password": "provided-out-of-band"
}
```

成功响应：

```json
{
  "token": "angui_...",
  "expires_at": "2026-07-13T20:00:00.000Z",
  "user": {
    "id": "uuid",
    "email": "family@demo.invalid",
    "display_name": "模拟家属",
    "account_type": "member",
    "global_capabilities": []
  }
}
```

连续失败登录会同时按客户端 IP 和 IP+邮箱组合限流。当前限流状态只保存在单个服务进程内。

## 4. 授权与可见性

- `member`：正常业务账号类型。可创建案件；创建者在新案件中获得 `family` 角色。只有被显式加入案件后才能读取或操作该案件。
- `learner`：长期学习账号类型；当前可登录并恢复会话，但不能创建或加入案件，学习与考核能力将在后续平台提供。
- `commander`：可叠加的全局能力。持有者可被显式授予某案件的 `commander` 角色；该案件角色才允许查看全部线索、审核线索、改变案件状态和添加成员。
- `volunteer`：可叠加的全局能力。持有者可被指挥显式授予某案件的 `volunteer` 角色；该案件角色只能查看已确认线索，老人资料中的健康注意字段由服务端删除。
- `admin`：可叠加的全局能力，不自动获得业务案件访问权，也不绕过案件成员关系。

无案件成员关系时，详情和操作接口返回 `404`，不会通过 `403` 暴露案件 ID 是否真实存在。角色已经是案件成员但动作不允许时返回 `403`。

账号响应中的 `account_type` 表示长期账号类型，`global_capabilities` 表示可叠加的平台资格；两者均不授予案件访问权。案件列表/详情中的 `access_role` 与成员响应中的 `case_role` 是仅对该案件有效的授权。`family` 来自建案或受控的显式成员操作；授予 `commander` 或 `volunteer` 案件角色时，目标 `member` 必须具有同名全局能力。具备多项能力的账号仍需逐案、逐角色地显式加入。

添加案件成员时，提交案件内角色：

```json
{
  "email": "volunteer@demo.invalid",
  "case_role": "volunteer"
}
```

## 5. 状态约束

案件状态为 `active`、`resolved`、`closed`。允许的变化为：

```text
active   -> resolved
active   -> closed
resolved -> active
resolved -> closed
```

相同状态的幂等更新允许通过。`closed` 案件不能再添加线索，也不能转回其他状态。

新线索始终由服务端写为 `pending_review`。人工审核接口只接受：

```text
needs_verification
confirmed
rejected
expired
duplicate
```

AI 或其他自动化能力未来只能生成草稿或待审核输入，不得绕过人工审核直接创建 `confirmed` 线索。

线索时间轴使用 `page`（默认 `1`）、`page_size`（默认 `25`，最大 `100`）、`status`、`sort`（`created_at` 或 `occurred_at`）和 `order`（`asc` 或 `desc`）查询参数。所有参数均为白名单；响应中的 `total` 只统计当前认证用户可见的线索。指挥可见全量，家属可见已确认或本人提交的线索，志愿者仅可见已确认线索；无案件成员关系一律返回 `404`。

## 6. 请求示例

创建案件：

```json
{
  "display_name": "模拟老人 A",
  "age": 76,
  "gender": "female",
  "physical_description": "短发，行动较慢",
  "clothing_description": "蓝色外套",
  "health_notes": "模拟认知障碍信息",
  "last_seen_at": "2026-07-13T09:00:00Z",
  "last_seen_location": "模拟公园北门"
}
```

`display_name` 和 `last_seen_location` 必填；年龄必须在 0 到 130 之间。未知字段会被拒绝。

提交线索：

```json
{
  "source": "family",
  "content": "模拟线索：曾向市场方向步行",
  "occurred_at": "2026-07-13T09:10:00Z",
  "location_text": "模拟公园北门"
}
```

审核线索：

```json
{
  "status": "confirmed"
}
```

更新案件状态：

```json
{
  "status": "resolved"
}
```

## Intake session creation

`POST /api/intake-sessions` is available only to authenticated `member` accounts. Its optional `initial_answers` object has eight structured draft fields: `basic_information`, `health_status`, `behavior_habits`, `last_seen`, `frequent_locations`, `belongings`, `transport_ability`, and `follow_up_clues`. Every supplied value is trimmed and must meet the active question's database-managed `max_answer_chars`; `ANGUI_INTAKE_ANSWER_HARD_MAX` is a server-side absolute cap (default `2000`, range `1`–`10000`) that cannot be exceeded by database configuration. Unknown properties, including `confirmed`, are rejected.

The server creates a `collecting` session and records the selected `question_set_version`. It always returns `guidance_mode: "rule_based"`, `missing_fields`, and the next ordered question from the active database definition. This is the required AI-unavailable fallback; no external model is called. The values remain unconfirmed drafts, never case facts. Raw answers are not included in audit metadata or ordinary logs. The session is owned by its creator; when a later confirmation associates it with a case, only an authorized commander of that case may additionally read it. The database's unique case association protects the later confirmation flow from linking a session to multiple cases.

Example:

```json
{
  "initial_answers": {
    "basic_information": "Fictional elder profile",
    "last_seen": "Fictional community gate; time needs verification"
  }
}
```

## Intake draft and confirmation

`GET /api/intake-sessions/{session_id}/profile-draft` returns the session creator's standardized profile draft. It is explicitly marked `draft`, lists its family-provided source scope and missing fields, includes a generated timestamp, and sets `requires_human_confirmation: true`. The endpoint does not infer a certain destination or expose the draft to commanders, volunteers, or other families before formal confirmation.

`POST /api/intake-sessions/{session_id}/confirm` is available only to that session creator after required answers are complete. The request must set `human_confirmed: true` and contains the family-reviewed profile, so corrected values supersede any draft value. One database transaction creates the active case, elder profile, creator's `family` membership, `case.created` audit event, and confirmed-session link. A repeat submission returns the already-created case instead of creating a duplicate. See `docs/openapi.yaml` for the exact schemas.

## 地点与图片补充

`POST /api/cases/{case_id}/places` 只允许该案件的 `family` 或 `commander` 成员调用。请求需要地点名称、由 `GET /api/cases/{case_id}/resource-configuration` 返回的地点类型、文字地址和 `public`、`confirmed` 或 `internal` 可见级别；经纬度必须同时提供，且服务端校验 longitude 在 -180..180、latitude 在 -90..90。地点来源由服务端按提交者的案件角色写入，客户端不能伪造。新地点的 `review_status` 初始为 `pending_review`，不会直接成为确认进展。志愿者不能通过此接口添加家庭地址或其他敏感地点。

`GET /api/cases/{case_id}/places` 只对案件成员开放，非成员统一返回 `404`。服务端按案件角色裁剪数据：指挥可查看案件全部地点；家属可查看本人提交的地点，以及已确认且非内部的地点；志愿者仅可查看已确认的公开地点。未审核地点和内部搜索方向不会通过该接口暴露给非必要角色。

`GET /api/cases/{case_id}/map-view` 是不依赖地图 SDK 的确定性态势接口。每个地图项都带对象类型、来源、时间、审核/任务状态、坐标或 `null` 与文字地点；无坐标记录保留在响应中作为文本回退。家属申报的最后出现地点始终标为 `pending_review`，其 `display_name` 为 `null`，客户端必须按 `object_type` 本地化标签而不能呈现为确认进展。家属不接收内部任务或线索层，志愿者只接收本人任务和已确认公开地点，指挥可额外查看已确认线索；接口不会返回预测位置。

`GET /api/cases/command-intake` 是指挥能力账号的待受理队列。它只返回尚无案件指挥、状态为 `active` 的案件编号、创建时间、最后出现时间、文字区域提示和老人年龄；不会返回姓名、病史、联系方式、原始线索、任务、附件、成员或精确坐标。`POST /api/cases/{case_id}/accept-command` 由指挥明确受理一项仍待受理的案件；服务端在同一事务中建立 `commander` 成员关系并写入 `case.commander_accepted` 审计事件，之后才返回完整的指挥角色裁剪案情。已有指挥受理的案件返回 `409`。

案件 `family` 成员可以添加另一名已注册的 `family` 成员或具备相应全局能力的 `commander`，但不能添加或调度 `volunteer`。该规则不替代指挥受理流程；家属不需要知晓或输入指挥账号来使案件进入待受理队列。

`GET /api/cases/{case_id}/members` 仅对该案件的 `commander` 成员开放，返回当前案件成员的展示名、案件角色与已授予的全局能力。它只服务于案件协作审查和人工选择已经授权的志愿者，不提供全局账号搜索，也不会泄露其他案件的成员关系；家属、志愿者和非成员分别按案件权限得到拒绝或 `404`。

`GET /api/cases/{case_id}/summary` 提供不依赖外部 AI 的确定性案件摘要。响应的 `generated_at` 和 `source_scope` 说明生成时间与当前角色可用的数据范围。只有人工审核为 `confirmed` 的线索才会进入 `last_confirmed_information` 与 `confirmed_clues`；`pending_review` 和 `needs_verification` 始终保留在 `pending_verification`，不会被表述为确认事实。指挥可见待核实事项、已排除方向、未完成任务形成的当前重点和全部任务状态；家属不接收任务或内部搜索方向；志愿者只接收本人任务的执行与安全信息。

`POST /api/tasks/{task_id}/feedback` 仅允许该任务受领志愿者在任务和案件均为 `active` 时提交。服务端将反馈与可选的本人上传附件写成关联任务的 `pending_review` 线索，并写入审计；反馈不会确认线索、改变案件事实或推进任务状态。

`POST /api/tasks/{task_id}/location-reports` 与 `POST /api/tasks/{task_id}/feedback` 都必须提供 UUID 格式的 `Idempotency-Key` 请求头。客户端为一次逻辑提交生成一个键，并在超时、断网等重试时复用该键；服务端会返回第一次成功提交的回执，不会重复写入位置、线索或审计事件。幂等键与规范化后的请求内容绑定：用同一键提交不同内容会返回 `409`，不会静默丢弃新的提交。

`GET /api/tasks/{task_id}/safety-briefing` 和 `GET /api/tasks/{task_id}/navigation` 仅允许该任务受领志愿者与案件指挥访问。安全提示是规则化的辅助信息，不是现场强制指令；即使实时天气等外部条件不可用，也会返回人工规则和紧急停止提示。导航接口只返回已授权任务区域的文字路线摘要；现有任务坐标没有坐标系声明时，服务端不会生成可能偏移的第三方导航链接。家属、其他志愿者和非成员不会获取这些内容。

## 公开进展、草稿与周边资源

`GET /api/cases/{case_id}/public-progress` 仅对该案件的 `family` 成员开放。它仅返回已人工审核为 `confirmed` 的进展类别、请求者本人仍待补充/核实的项目类别、以及安全和联系提醒；它绝不返回任何原始线索正文、未核实的他人线索、内部搜索方向、任务与分配、志愿者位置、病史全文或成员详情。每项仅包含服务端生成的公开类别、审核状态和更新时间，客户端不得把其他案件详情替代为“公开进展”。

`POST /api/cases/{case_id}/clue-drafts` 可由案件 `family`、`commander` 或 `volunteer` 调用，用于将聊天/文本整理成持久化的 `draft`。响应会保留草稿 ID、受控原始记录引用、模板版本、可选模型标识和不确定性提示。服务不可用时以 `rule_based_fallback` 返回，不阻断人工工作；该接口不创建正式线索，更不允许创建 `confirmed` 线索。

`GET /api/cases/{case_id}/pois` 只允许 `commander` 和 `volunteer`，且客户端只能选择医院、派出所、公交站、市场或社区服务中心等白名单类别。中心坐标只能由服务端从当前角色可见的任务点或指挥已确认地点选取；单次查询固定为 3 km、单页、最多 10 项。高德 Web 服务的 key 仅保存在服务端；HTTP 或业务状态失败时，响应将标记为 `degraded` 并给出固定的虚构非坐标回退结果，绝不泄露 key、上游 URL 或案件中心坐标。

`GET /api/cases/{case_id}/summary-drafts`、`POST /api/cases/{case_id}/summary-drafts` 和 `PATCH /api/cases/{case_id}/summary-drafts/{draft_id}/review` 仅对 `commander` 开放。指挥完成一条线索的人工审核后，服务端会自动创建一份 `pending_review` 摘要草稿；页面只显示最新待审核草稿，指挥只需发布或驳回。`POST` 保留为内部重生成能力，而不是普通页面入口。生命周期包含 `draft`、`pending_review`、`published`、`rejected`、`withdrawn`、`superseded`。每次提交、审核、发布或撤回都记录操作者、理由、时间和版本；发布会将该案件既有的已发布版本标为 `superseded`。当前 AI 网关只完成提供方合规路由和协议请求构造，尚未执行外部模型推理，因此摘要一律使用同一受控来源范围的确定性降级草稿；未来实际推理的所有提供方失效时也沿用该降级路径。降级不会阻塞线索审核，草稿仍必须人工审核后才能发布，且不会被标记为某个 AI 模型的输出。只有服务端依据受控来源范围生成的 `pending_review` 草稿可发布；自由文本草稿仅限内部使用，审核阶段不可用请求内容覆盖草稿，从而避免把未审核线索、内部任务或健康字段发布给非必要角色。

`POST /api/cases/{case_id}/archive-drafts` is commander-only and accepts no client-supplied case material. It is available only after a case reaches `resolved` or `closed`. The server creates an internal `draft` from allowlisted status and count metadata for confirmed clues and completed tasks, persists its explicit `source_scope`, and marks `deidentification_status` as `manual_review_required`. It does not copy raw clue text, attachments, health notes, contacts, exact locations, routes, or task results. This endpoint does not publish, index, export, or print material; a separate authorized de-identification, review, and withdrawal workflow is required before any later reuse.

`POST /api/cases/{case_id}/attachments` 使用 `multipart/form-data` 的单个 `file` 字段。首版只接收 MIME 声明（允许带参数）与文件魔数一致的 JPEG/PNG。服务端解码并重新编码图片以移除 EXIF/GPS 等非必要元数据，使用随机且不可猜测的存储键保存，并在元数据或审计写入失败时删除刚写入的文件。存储目录由 `ANGUI_ATTACHMENT_STORAGE_DIRECTORY` 配置，默认 `data/attachments`，不能包含 `..` 路径分段，且必须位于静态公开目录外。下载响应包含 `X-Content-Type-Options: nosniff` 和 `Cache-Control: no-store, private`。

`POST /api/clues/{clue_id}/attachments` uses the same single-file JPEG/PNG validation, normalization, EXIF/GPS removal, byte limit, and protected storage policy. The attachment row, the clue-to-attachment link, and the audit event are committed together. A commander may add evidence to any clue in the case; family and volunteer members may only add evidence to clues they submitted. The new image remains `pending_review`, and ordinary attachment download rules still prevent family or volunteers from reading another member's internal evidence.

以下限制均由启动配置统一控制，服务层和 multipart 读取层会使用同一份值：

- `ANGUI_ATTACHMENT_MAX_IMAGE_BYTES`：单图字节上限，默认 `5242880`（5 MiB），允许 `1024` 至 `20971520`。
- `ANGUI_ATTACHMENT_MAX_PER_CASE`：单案件附件数量上限，默认 `12`，允许 `1` 至 `100`。
- `ANGUI_CASE_PLACE_TYPES`：逗号分隔的允许地点类型，默认 `frequent,key_location,last_seen_context,medical,shelter,other`；每项只允许小写字母、数字和下划线，最多 16 项，不可重复。

附件下载必须使用带 Bearer 会话的 `GET /api/cases/{case_id}/attachments/{attachment_id}`。指挥可下载该案件附件；家属和志愿者只能下载自己提交的附件，因此家属不会读取志愿者内部反馈图片。所有地点、图片、文字线索和时间补充都从 `pending_review` 开始，客户端不得将其呈现为已确认进展。

## 7. 错误响应

错误统一使用：

```json
{
  "error": {
    "code": "validation_error",
    "message": "last_seen_location is required"
  }
}
```

当前状态码映射：

- `400 validation_error`：请求字段或状态值无效。
- `401 unauthorized`：没有会话、令牌无效、会话过期或已撤销。
- `403 forbidden`：用户是案件成员，但当前角色不能执行该动作。
- `404 not_found`：案件或线索不存在。
- `409 conflict`：状态转换冲突，或向非进行中案件添加线索。
- `429 rate_limited`：登录失败次数超过当前进程限制。
- `500 database_error`：数据库操作失败；响应不会返回内部 SQL。

## 8. 事务与审计

以下审计事件与对应业务写入在同一数据库事务中提交：

- `case.created`
- `case.status_changed`
- `clue.submitted`
- `clue.reviewed`
- `case.member_added`
- `auth.login_succeeded`
- `auth.login_failed`
- `auth.logout`

审计 actor 来自认证用户 ID。登录失败不会把邮箱或密码写入审计元数据；密码和 Bearer 令牌禁止进入日志。账号初始化 CLI 每次更新演示账号密码时会撤销该账号的既有会话。

## 管理员账号管理

`admin` 是平台级 capability，只授权 `/api/admin/*` 下的管理员接口；它不会自动成为任何案件的成员，也不会绕过案件级权限检查。管理员接口不提供创建、角色或 capability 变更功能。

`GET /api/admin/audit-events` 支持按案件 ID、实体类型、事件 action、RFC 3339 时间范围以及白名单排序分页查询。响应只包含审计事件标识、关联案件/实体、操作人、事件类别和时间，不返回 `metadata_json`、原始请求内容或任何敏感业务详情；每次访问本身也会被审计。

`GET /api/admin/users` 只返回账号 ID、邮箱、展示名、账号类型、全局 capability、状态、创建时间和最近会话时间。密码哈希、Bearer token、token 哈希、会话 ID、画像及案件资料绝不会通过该接口返回。筛选和排序参数均为服务端白名单。

`PATCH /api/admin/users/{user_id}/status` 只接受 `active`、`disabled` 或 `locked`，并要求提供审核理由。将账号设为 `disabled` 或 `locked` 时，服务端在同一事务中撤销其全部活跃会话，因此既有 token 立即失效；重新设为 `active` 不会恢复已撤销的旧会话，用户必须重新登录。为避免管理员误锁自己，管理员不能修改自己的账号状态。审计仅记录操作人、变更前后状态、撤销会话数和理由长度，不保存理由原文或凭据。

## 归档脱敏与经验案例审核

归档草稿始终从仅含状态和计数元数据的 `draft` 开始；原始聊天、完整身份、联系方式、病史、精确地点/轨迹、附件、线索正文和任务结果文本不进入草稿。`POST /api/admin/archive-drafts/{draft_id}/deidentify` 与 `PATCH /api/admin/archive-drafts/{draft_id}/review` 均仅对具有 `admin` capability 的账号开放，且不会因该 capability 自动获得源案件的读取权限。

`POST /api/admin/archive-drafts/{draft_id}/deidentify` 只接受人工 `confirm` 或 `reject` 结果和长度受限的理由。它不接受替换文本、原始材料或“自动完全脱敏”的声明。确认会把草稿转为 `pending_review` 和 `deidentified`；拒绝会转为 `rejected`，不能发布。每次结果记录操作者、时间、版本和理由长度，审计不保存理由原文。

`PATCH /api/admin/archive-drafts/{draft_id}/review` 只允许已确认脱敏的 `pending_review` 草稿 `publish` 或 `reject`，以及将 `published` 草稿 `withdraw`。发布仅把该受控版本标记为 `learning_resource`；当前不会创建知识问答/RAG 索引、导出、打印、公开读取或跨案件读取接口。撤回立即将用途恢复为 `internal_archive` 并标记 `withdrawn`，同时保留版本和审计历史；拒绝或撤回绝不修改或删除原案件。
