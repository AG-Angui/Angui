# API 说明

## 1. 当前范围

当前 API 提供用于 MVP 开发的认证、案件成员授权、案件和线索纵向闭环。它已经连接 SeaORM 数据层，并实现可撤销数据库会话和服务端字段裁剪；密码找回、MFA、账号审批、组织模型和生产级安全运营仍未实现。请求只允许使用虚构或充分脱敏的数据。

默认地址为 `http://127.0.0.1:8080`，所有业务接口位于 `/api` 下。

## 2. 接口列表

| 方法 | 路径 | 成功状态 | 用途 |
| --- | --- | --- | --- |
| `GET` | `/api/health` | `200` | 服务健康检查 |
| `POST` | `/api/auth/login` | `200` | 使用邮箱和密码创建短期会话 |
| `GET` | `/api/auth/me` | `200` | 获取当前认证用户 |
| `POST` | `/api/auth/logout` | `204` | 撤销当前服务端会话 |
| `GET` | `/api/cases` | `200` | 按创建时间倒序列出案件 |
| `POST` | `/api/cases` | `201` | 创建案件和老人画像 |
| `GET` | `/api/cases/{case_id}` | `200` | 查询案件、老人画像和线索 |
| `PATCH` | `/api/cases/{case_id}/status` | `200` | 人工更新案件状态 |
| `POST` | `/api/cases/{case_id}/clues` | `201` | 提交待审核线索 |
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
    "role": "family"
  }
}
```

连续失败登录会同时按客户端 IP 和 IP+邮箱组合限流。当前限流状态只保存在单个服务进程内。

## 4. 授权与可见性

- `family`：可创建案件；只能访问本人作为成员的案件；可看已确认线索和本人提交的待审核线索；可按邮箱显式邀请一名全局角色为指挥的账号；不能添加其他角色、审核线索或改变案件状态。
- `commander`：必须是案件成员；可查看全部线索、审核线索、改变案件状态并添加案件成员。
- `volunteer`：必须由指挥加入案件；只能查看已确认线索，老人资料中的健康注意字段由服务端删除。
- `admin`：管理角色不自动获得业务案件访问权。

无案件成员关系时，详情和操作接口返回 `404`，不会通过 `403` 暴露案件 ID 是否真实存在。角色已经是案件成员但动作不允许时返回 `403`。

添加案件成员时，请求角色必须与目标账号的全局角色一致：

```json
{
  "email": "volunteer@demo.invalid",
  "role": "volunteer"
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
