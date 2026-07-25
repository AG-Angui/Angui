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
| `POST` | `/api/cases` | `201` | 创建案件和老人画像 |
| `GET` | `/api/cases/{case_id}` | `200` | 查询案件、老人画像和线索 |
| `PATCH` | `/api/cases/{case_id}/status` | `200` | 人工更新案件状态 |
| `GET` | `/api/cases/{case_id}/clues` | `200` | 获取角色裁剪、可分页的线索时间轴 |
| `POST` | `/api/cases/{case_id}/clues` | `201` | 提交待审核线索 |
| `POST` | `/api/cases/{case_id}/places` | `201` | 家属或指挥提交常去/关键地点，始终待人工审核 |
| `GET` | `/api/cases/{case_id}/resource-configuration` | `200` | 获取当前案件可用的地点类型和图片限制 |
| `POST` | `/api/cases/{case_id}/attachments` | `201` | 上传受控 JPEG/PNG 案件图片，始终待人工审核 |
| `GET` | `/api/cases/{case_id}/attachments/{attachment_id}` | `200` | 按案件权限下载本人上传的图片，指挥可下载案件全部图片 |
| `POST` | `/api/cases/{case_id}/members` | `201` | 家属邀请指挥，或指挥添加案件成员 |
| `PATCH` | `/api/clues/{clue_id}/review` | `200` | 人工审核线索 |

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

`POST /api/cases/{case_id}/attachments` 使用 `multipart/form-data` 的单个 `file` 字段。首版只接收 MIME 声明（允许带参数）与文件魔数一致的 JPEG/PNG。服务端解码并重新编码图片以移除 EXIF/GPS 等非必要元数据，使用随机且不可猜测的存储键保存，并在元数据或审计写入失败时删除刚写入的文件。存储目录由 `ANGUI_ATTACHMENT_STORAGE_DIRECTORY` 配置，默认 `data/attachments`，不能包含 `..` 路径分段，且必须位于静态公开目录外。下载响应包含 `X-Content-Type-Options: nosniff` 和 `Cache-Control: no-store, private`。

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
- `409 conflict`：状态转换冲突，或向已关闭案件添加线索。
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
