# 安归实时协作空间产品与技术方案

## 文档范围

本文只描述当前尚未实现、需要新增或改造的“实时协作空间”能力。协作空间不是案件管理页面，也不是家属、指挥、志愿者三端平台的统称；它是绑定单一案件的实时行动房间，用于现场成员的位置协同、任务执行、文字/语音交流和语音线索上报。

## 1. 产品简述

一个协作空间只关联一个案件。指挥员和志愿者可以加入空间；志愿者加入时需要明确同意位置轨迹采集与空间内共享。

在同一活动协作空间中：

- 指挥员可查看所有成员的实时位置、状态、轨迹和到达事件。
- 志愿者可查看空间内其他志愿者的实时位置；历史轨迹默认仅展示当前行动窗口内的简化数据。
- 志愿者可查看自己的任务，也可接收指挥员通过协作空间下发的任务。
- 成员可进行文字交流；后续可增加实时语音交流。
- 志愿者可录制语音上报，系统通过 ASR 转写并由 AI 提取候选线索，最终仍由指挥员审核。

家属不加入协作空间，也不读取协作空间中的实时位置、轨迹、内部聊天或未审核语音线索。

## 2. 硬性业务约束

| 约束 | 设计要求 |
| --- | --- |
| 空间与案件 | 一个 `collaboration_space` 只关联一个 `case_id`。建议首版限制一个案件同时仅有一个 `active` 空间，归档空间用于复盘。 |
| 志愿者并发加入 | 一个志愿者最多同时加入 3 个 `active` 空间。限制必须通过数据库事务校验，不能只依赖 Redis 计数。 |
| 位置共享 | 每个空间成员关系必须有单独的位置授权记录；加入、撤回、退出、空间关闭都会影响共享状态。 |
| 位置可见性 | 仅向同一空间内的授权成员展示；禁止跨空间、跨案件或向家属暴露。 |
| 事实确认 | ASR 转写、AI 提取和语音上报都是候选材料，不能直接确认线索、派发任务或公开发布。 |
| 审计 | 加入空间、位置授权、位置撤回、任务派发、消息删除、语音线索审核必须可审计。 |

## 3. 待建设的产品能力

### 3.1 空间创建、加入与退出

指挥员为案件创建协作空间，空间可设置名称、行动时间、集合点、搜索范围、最大人数和状态。

志愿者进入空间时需要完成：

1. 阅读本空间的位置共享说明。
2. 明确授权当前位置和行动期间轨迹的采集与共享。
3. 查看自己已加入的活动空间数量；达到 3 个时不能继续加入。
4. 进入后开始发送心跳和位置状态。

退出空间或撤销授权后，客户端必须立即停止该空间的位置上报；服务端必须停止向其他成员广播该用户的最新位置。

### 3.2 实时地图、轨迹与到达事件

协作空间地图需要支持：

- 全部在线成员的最新位置、最后回传时间、定位精度和在线状态。
- 选中某位成员后查看当前行动窗口内的轨迹。
- 展示任务点、集合点、搜索区域和危险区域。
- 显示成员首次进入任务点、集合点或任务区域的到达时间。
- 显示位置已过期、定位精度低、离开任务区或长时间无回传等安全状态。

到达事件必须由服务端计算并去重，不能仅接受客户端提交的“我已到达”。判断可基于目标点半径或任务多边形：

```text
position.updated
  -> 校验成员、授权与定位精度
  -> 判断是否首次进入目标点半径或任务区域
  -> 去抖，避免 GPS 漂移重复触发
  -> 记录 point_arrival
  -> 发布 member.arrived
```

### 3.3 任务协同

任务的正式业务事实仍属于案件任务，但需要新增“任务在协作空间中的执行上下文”。

- 指挥员可在空间中创建、选择或下发任务。
- 任务可指定成员、指定小组或开放认领。
- 志愿者可在空间中接受、开始、阻塞、完成任务。
- 任务详情应包含区域、风险、装备、联系人、到达条件、反馈要求和安全提示。
- 任务状态变化要同步到空间地图、任务面板和相关成员。

### 3.4 文字与语音交流

文字消息用于现场协调、集合通知、任务追问和状态同步，应具备发送、撤回、引用、已读/送达状态和审计记录。

语音分为两类：

| 类型 | 目标 | 建议技术 | 默认保留策略 |
| --- | --- | --- | --- |
| 实时语音交流 | 成员之间即时联络 | WebRTC | 默认不录音、不长期保存；录音需单独授权 |
| 语音上报 | 形成可审核的现场线索 | 音频上传 + 对象存储 + ASR | 保留受控原音频、转写、候选草稿和审核记录 |

### 3.5 语音上报、ASR 与 AI 提取

语音上报工作流必须是“候选生成 + 人工审核”，而不是自动入库：

```text
志愿者录音
  -> 上传音频对象
  -> 创建 voice_report
  -> 异步 ASR 转写
  -> 保存 transcript
  -> AI 提取时间、地点、人物、方向、行为、置信度和缺失字段
  -> 创建 clue_draft
  -> 指挥员审核、编辑、确认或排除
  -> 形成正式 clue
```

语音原文件和转写文本必须有独立权限控制；AI 不得直接向其他成员广播未经审核的推断结论。

## 4. 领域模型与数据表

建议新增以下模型。表名只表达目标领域，具体命名可遵循现有数据库迁移规范。

| 表/模型 | 核心字段 | 作用 |
| --- | --- | --- |
| `collaboration_spaces` | `id`、`case_id`、`name`、`status`、`created_by`、`started_at`、`ended_at` | 协作空间；`case_id` 为外键 |
| `space_members` | `space_id`、`user_id`、`role`、`status`、`joined_at`、`left_at` | 空间成员关系；角色仅允许 commander/volunteer |
| `space_location_consents` | `space_id`、`user_id`、`consent_version`、`granted_at`、`revoked_at` | 单独记录位置共享同意与撤回 |
| `space_location_samples` | `space_id`、`user_id`、`latitude`、`longitude`、`accuracy_meters`、`captured_at` | 持久化、可回放的轨迹点 |
| `space_arrivals` | `space_id`、`user_id`、`target_type`、`target_id`、`arrived_at`、`accuracy_meters` | 到达集合点、任务点或任务区域的事件 |
| `space_messages` | `space_id`、`sender_id`、`message_type`、`content`、`sent_at`、`recalled_at` | 可审计文字消息和指挥广播 |
| `voice_reports` | `space_id`、`case_id`、`reporter_id`、`object_key`、`status`、`created_at` | 原始语音上报的受控入口 |
| `voice_transcripts` | `voice_report_id`、`content`、`asr_provider`、`status`、`completed_at` | ASR 转写结果 |
| `space_events` | `event_id`、`space_id`、`case_id`、`event_type`、`version`、`visibility_scope`、`payload_json` | 可补偿、可审计的空间事件 |
| `event_outbox` | `id`、`event_id`、`status`、`attempt_count`、`available_at` | 数据库事务提交后的可靠分发队列 |

### 4.1 数据库约束

- `collaboration_spaces.case_id` 必须存在；首版建议建立“每案件仅一个活动空间”的条件唯一约束或服务端事务校验。
- `space_members(space_id, user_id)` 必须唯一。
- `space_members.role` 只允许 `commander`、`volunteer`。
- 加入空间时，服务端在同一事务中锁定用户的活动成员关系，统计其 `active` 空间数；数量达到 3 时返回冲突。
- 位置样本只允许由该空间中具有有效同意记录的活动志愿者写入。
- 任务、线索、语音报告和空间成员必须属于同一案件，禁止使用 ID 拼接绕过案件边界。
- 轨迹点、到达事件和消息需要配置保留期与归档/删除流程。

## 5. 实时架构

### 5.1 职责划分

```text
客户端
  -> HTTP API：正式命令、查询、文件上传
  -> WebSocket：实时位置、在线状态、空间事件

业务服务
  -> PostgreSQL：正式事实、授权、审计、轨迹历史、任务、消息
  -> Event Outbox：事务后的可靠事件分发
  -> Redis：在线状态、最新位置、短期限流、跨实例广播
  -> 对象存储：语音和附件
  -> Worker：轨迹批量写入、ASR、AI 提取、过期清理
```

### 5.2 Redis 是否需要引入

需要，但不需要首版就上 Redis Cluster。

Redis 的作用是解决多 WebSocket 实例之间的实时状态共享和事件扇出，而不是保存案件的最终事实。单机开发或单实例部署可以暂时使用进程内广播，但只要 WebSocket 网关需要横向扩容，Redis 就应成为基础依赖。

Redis 只保存短生命周期数据：

```text
space:{space_id}:member:{user_id}:presence
space:{space_id}:channel
space:{space_id}:online-members
user:{user_id}:active-spaces
rate-limit:{user_id}:location
```

`presence` 应设置 TTL，例如 45 秒。没有新的定位或心跳时，成员状态自动变为位置过期或离线；Redis 重启不会导致案件、任务、轨迹历史或审核结果丢失，因为这些都不以 Redis 为事实源。

### 5.3 Redis 压力控制

Redis 压力的关键不是“志愿者可以加入 3 个空间”，而是是否无节制地采样、持久化和广播位置。

位置更新策略：

- 前台行动中，每 5 至 15 秒采样一次；移动超过 10 至 30 米时可提前采样。
- 静止、低电量、后台运行或弱网时自动降低频率。
- 同一设备只获取一次 GPS，再分发给该用户最多 3 个已授权空间。
- Redis 只保存最新点；历史轨迹异步批量落 PostgreSQL。
- WebSocket 广播只发送“某成员的新最新位置”，不发送整个空间的成员列表或完整轨迹。
- 客户端默认只渲染当前点；打开成员详情时才请求时间范围内的历史轨迹。

估算公式：

```text
输入位置事件/秒 = 活动成员关系数量 / 平均上报间隔秒
广播投递/秒 = 输入位置事件/秒 * (空间平均在线接收者数 - 1)
```

例如，20 个空间、每空间 50 名成员、每 10 秒更新一次：

```text
输入位置事件约 = 1000 / 10 = 100 次/秒
广播投递约 = 100 * 49 = 4900 次/秒
```

该量级通常不是 Redis 的瓶颈。优先出现压力的地方往往是 WebSocket 出站带宽、地图渲染，以及每点同步写数据库。需要在单空间成员达到约 100 至 200 人、位置频率降到 3 秒以内、或 WebSocket 网关横向扩容后，再评估 Redis 分片/集群和专用实时网关。

### 5.4 可靠事件分发

Redis Pub/Sub 不具备离线补偿能力，不能单独作为可靠事件日志。正式事件必须先写入数据库：

```text
HTTP 命令
  -> 数据库事务：业务记录 + 审计记录 + event_outbox
  -> outbox worker 读取待投递事件
  -> 发布 Redis channel
  -> WebSocket Gateway 推送给授权成员
  -> 标记 outbox 已投递或等待重试
```

客户端断线恢复时不依赖 Redis 历史，而是：

```text
重新连接
  -> 获取 collaboration space snapshot
  -> 带 after_version 请求缺失事件
  -> 版本不连续或游标失效时重新拉快照
```

## 6. API 与 WebSocket 协议建议

### 6.1 HTTP API

```text
POST   /api/cases/{case_id}/collaboration-spaces
GET    /api/cases/{case_id}/collaboration-spaces
GET    /api/collaboration-spaces/{space_id}
POST   /api/collaboration-spaces/{space_id}/join
POST   /api/collaboration-spaces/{space_id}/leave
POST   /api/collaboration-spaces/{space_id}/location-consents
DELETE /api/collaboration-spaces/{space_id}/location-consents/me
GET    /api/collaboration-spaces/{space_id}/snapshot
GET    /api/collaboration-spaces/{space_id}/members/{user_id}/track
GET    /api/collaboration-spaces/{space_id}/members/{user_id}/arrivals
POST   /api/collaboration-spaces/{space_id}/tasks
POST   /api/collaboration-spaces/{space_id}/messages
POST   /api/collaboration-spaces/{space_id}/voice-reports
GET    /api/collaboration-spaces/{space_id}/events?after_version={version}
```

### 6.2 WebSocket

```text
WS /api/realtime/collaboration-spaces/{space_id}
```

客户端发送：

```json
{
  "type": "location.update",
  "operation_id": "uuid",
  "latitude": 23.1291,
  "longitude": 113.2644,
  "accuracy_meters": 18,
  "captured_at": "2026-08-15T10:20:30Z"
}
```

服务端向授权成员推送：

```json
{
  "event_id": "uuid",
  "space_id": "space_xxx",
  "case_id": "case_xxx",
  "event_type": "member.location_updated",
  "version": 241,
  "occurred_at": "2026-08-15T10:20:31Z",
  "visibility_scope": "space_members",
  "payload": {
    "user_id": "user_xxx",
    "latitude": 23.1291,
    "longitude": 113.2644,
    "accuracy_meters": 18,
    "captured_at": "2026-08-15T10:20:30Z"
  }
}
```

核心事件名称：

```text
space.member_joined
space.member_left
space.location_consent_granted
space.location_consent_revoked
member.location_updated
member.location_expired
member.arrived
task.dispatched
task.accepted
task.blocked
task.completed
message.sent
message.recalled
voice_report.created
voice_report.transcribed
clue_draft.created
clue_draft.reviewed
```

## 7. 客户端待建设能力

### 7.1 志愿者移动端

- 活动协作空间列表，明确显示当前已加入空间数及上限 3。
- 加入前的位置授权说明和撤回入口。
- 地图成员实时点、本人任务、任务区域、集合点和危险区。
- 成员选择后的简化轨迹与到达记录。
- 固定在底部的“上报线索”“任务状态”“离开空间”操作。
- 位置、消息和语音上报的本地 outbox；恢复网络后带 `operation_id` 重试。

### 7.2 指挥端 PC

- 空间成员列表、在线状态、最后回传时间和异常筛选。
- 地图中心的成员位置、轨迹回放、任务区、集合点和危险区。
- 任务派发/调整、文字广播、语音上报审核入口。
- 空间状态控制：开始行动、暂停、关闭、归档。
- 空间级安全告警：成员越界、位置过期、定位精度异常、任务超时。

### 7.3 共享前端基础设施

- WebSocket 连接管理、断线重连、心跳、事件去重和版本补偿。
- 空间快照缓存与事件增量合并。
- 位置采样器：频率自适应、移动阈值、低电量降频和授权检查。
- 本地 outbox：消息、位置、任务反馈、语音报告的幂等重试。
- 地图图层：成员当前点、成员轨迹、任务区、到达点、危险区。

## 8. 语音与 AI Worker 设计

### 8.1 语音上报处理队列

```text
voice_report.created
  -> Worker 获取对象存储音频
  -> ASR 转写
  -> 写入 voice_transcripts
  -> 发布 voice_report.transcribed
  -> AI 结构化提取
  -> 写入 clue_draft
  -> 发布 clue_draft.created
```

Worker 应具备：

- 可重试任务和幂等键。
- 音频时长、格式、大小和内容类型校验。
- 转写失败、模型失败和超时后的明确状态，不伪造成功结果。
- 仅将最小、受控材料发送给 ASR/AI 服务。
- 原始音频、转写、AI 候选、人工修改和最终线索之间的审计关联。

### 8.2 RAG 边界

实时协作空间中的聊天、位置、原始语音、未审核转写和未审核线索，均不得进入 RAG 索引。只有案件结束后、经脱敏和人工审核发布的案例材料，才能成为学习知识库的一部分。

## 9. 安全、隐私与保留策略

- 位置共享必须按“空间成员关系”逐一同意，而不是全局一次授权后永久共享。
- 撤销位置同意后，立即停止新位置采集与实时广播；历史轨迹是否继续保留取决于保留策略和审计要求。
- 轨迹、消息、语音和转写都需要按空间/案件权限校验，不能通过对象 ID 直接访问。
- 原始语音和附件必须使用不可猜测对象键、服务端授权下载、过期签名或 Bearer 会话访问。
- 轨迹保留期、消息保留期、语音保留期和归档策略需通过配置管理，避免无限存储。
- Redis 必须仅内网可达，启用认证、TLS、内存上限、TTL 和监控；Redis 不保存长期敏感材料。

## 10. 实施顺序与验收

### P0：空间和位置基础

- 新增空间、成员、位置授权、位置样本、到达事件和 outbox 数据迁移。
- 实现空间创建/加入/退出/授权/撤回 API。
- 实现空间快照、活动成员上限 3 的数据库事务校验。
- 实现志愿者位置上报、Redis 最新位置、WebSocket 当前点同步。
- 实现指挥端成员位置视图和志愿者端自己的空间状态。

验收：空间内的授权志愿者能互看实时位置；退出或撤回同意后，其他成员不再看到该志愿者的实时点；第四个活动空间加入请求被拒绝。

### P1：轨迹、任务和文字协同

- 异步批量持久化轨迹点，按成员和时间查询轨迹。
- 实现集合点/任务区到达事件。
- 将案件任务关联到协作空间，支持派发、接受、阻塞、完成。
- 实现文字消息、指挥广播和送达/已读状态。
- 实现断线重连、版本补偿和客户端 outbox。

验收：选中成员可以查看当前行动窗口内的轨迹和到达时间；任务状态实时同步；弱网恢复后不会生成重复消息、重复位置或重复任务反馈。

### P2：语音、ASR 与线索审核

- 实现受控音频上传、对象存储和 voice report 生命周期。
- 实现 ASR Worker、转写状态、失败重试和状态通知。
- 实现 AI 结构化提取并创建待审核 clue draft。
- 在指挥端实现转写查看、候选编辑、确认/排除和审计链。

验收：语音上报在 ASR/AI 失败时仍可追踪失败原因；任何 AI 输出均不能绕过人工审核成为正式线索。

### P3：实时语音和规模化

- 若现场确有强需求，再引入 WebRTC 实时语音房间。
- 评估 SFU、TURN、录音授权、网络降级和移动端后台限制。
- 根据活动空间数、成员数、更新频率和 WebSocket 网关数量评估 Redis 分片或集群。

验收：实时语音不影响位置、任务和文字消息的核心可用性；Redis 或语音服务异常时，HTTP 任务操作和案件事实仍可正常恢复。

## 11. 当前不应实施的事项

- 不将 Redis 用作案件、任务、轨迹历史或审核结果的唯一存储。
- 不将位置向全平台志愿者、跨空间成员或家属公开。
- 不在首版把完整历史轨迹持续广播给每个成员。
- 不将实时语音通话和语音线索上报混为一个功能。
- 不让 ASR 或 AI 自动确认线索、自动派发任务、自动公开进展。
- 不在没有规模数据前直接建设 Redis Cluster、媒体 SFU 或复杂多区域部署。
