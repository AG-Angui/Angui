# API 说明

## 1. 当前范围

当前 API 提供用于 MVP 开发的案件和线索纵向闭环。它已经连接 SeaORM 数据层，但尚未实现正式登录、角色授权、字段级权限、限流或生产级隐私保护。请求只允许使用虚构或充分脱敏的数据。

默认地址为 `http://127.0.0.1:8080`，所有业务接口位于 `/api` 下。

## 2. 接口列表

| 方法 | 路径 | 成功状态 | 用途 |
| --- | --- | --- | --- |
| `GET` | `/api/health` | `200` | 服务健康检查 |
| `GET` | `/api/cases` | `200` | 按创建时间倒序列出案件 |
| `POST` | `/api/cases` | `201` | 创建案件和老人画像 |
| `GET` | `/api/cases/{case_id}` | `200` | 查询案件、老人画像和线索 |
| `PATCH` | `/api/cases/{case_id}/status` | `200` | 人工更新案件状态 |
| `POST` | `/api/cases/{case_id}/clues` | `201` | 提交待审核线索 |
| `PATCH` | `/api/clues/{clue_id}/review` | `200` | 人工审核线索 |

## 3. 状态约束

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

## 4. 请求示例

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

## 5. 错误响应

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
- `404 not_found`：案件或线索不存在。
- `409 conflict`：状态转换冲突，或向已关闭案件添加线索。
- `500 database_error`：数据库操作失败；响应不会返回内部 SQL。

## 6. 事务与审计

以下审计事件与对应业务写入在同一数据库事务中提交：

- `case.created`
- `case.status_changed`
- `clue.submitted`
- `clue.reviewed`

当前 actor 值 `demo:family`、`demo:commander` 是开发占位符，不代表服务已经认证调用者。接入正式身份系统后必须从可信服务端身份上下文生成 actor，并执行案件关系和角色权限检查。
