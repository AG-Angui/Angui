# 学习内容治理操作说明

## 分类与标签治理（#193）

- `learning_categories` 是分类的唯一治理来源：学习者只能通过 `POST /api/learning/categories/proposals` 提交 `pending` 申请；管理员以带理由的 `enable`、`reject`、`disable` 操作处理。分类名称会压缩内部空白并以大小写无关的规范键去重。
- 学习资源保留原有 `tags_json`，以便既有数据和三数据库部署平滑兼容；服务端对标签压缩空白、按规范键去重，并限制为最多 12 个。资源列表支持 `category_id` 与 `tag` 的 AND 组合筛选。
- `learning_resources.category_id` 和 `category_name` 均可为空。旧资源无需回填，仍可读、可搜索；新资源只能选择 `enabled` 分类。分类被停用后不会回写或隐藏历史资源，响应使用资源写入时的分类名称快照，保证版本和审计可追溯。
- 学习者只能从学习中心调用 `POST /api/learning/resources/drafts`，且服务端强制 `visibility=learner` 与 `permitted_use=training`。该端点只写入 `submitted` 事件，之后仍必须由不同管理员完成去标识、审核和发布；它不会提供任何绕过内容治理的发布能力。
- 分类的申请与状态变更写入专用 `learning_category_review_events`，并写入不含资源正文、案例内容或个人信息的总审计事件。分类审计只记录分类 ID、动作、操作者、时间和受限理由。

### 迁移与回滚

迁移 `m0044_add_learning_category_governance` 为 SQLite、PostgreSQL、MySQL 同步创建分类和分类审计表，并为资源追加两个可空列与索引。向上迁移不改写历史资源。若分类表已写入数据，或任一资源已经关联分类，向下迁移会被 Rust 的安全检查拒绝；应先按数据保留策略导出/归档并显式清理，而不是在生产库中强制回滚。

学习中心不会从案件原始资料、聊天记录、完整身份信息、联系方式、病史或精确轨迹自动生成可读内容。学习资源和题目必须由管理员通过受控接口录入，并依次完成脱敏、独立审核和发布。

## 状态与职责

1. 管理员提交资源或题目：`POST /api/admin/learning/resources` 或 `POST /api/admin/learning/questions`。提交后内容为不可见状态。
2. 非提交人管理员确认脱敏：`POST .../{id}/deidentify`。理由必须记录。
3. 非提交人管理员完成审核：`POST .../{id}/review`。审核前不能发布。
4. 管理员发布：`POST .../{id}/publish`。题目的来源资源必须已经完成同一治理链并处于已发布状态。
5. 管理员撤回：`POST .../{id}/withdraw`。撤回会立即从资源列表、题库、问答与答题流程中移除内容，历史治理事件和审计记录保留。

所有治理请求体均为：

```json
{ "reason": "可审计的人工处理理由" }
```

资源提交额外要求来源名称、可选 HTTPS 来源地址、版本生效时间、可见级别和用途。资源的 `permitted_use` 仅允许 `training` 或 `public_information`；两种用途均须完成同一脱敏、独立审核与发布链后才可读取或导出。题目及其判题、解析固定为 `training`，不将公开信息用途扩展为题库用途。

## 读取与导出边界

- `GET /api/learning/public/prevention-card` 无需登录，但只返回当前已发布、已完成治理、公开可见且用途为培训或公开信息的防走失知识卡。生产环境仅缓存这一张卡和前端应用壳，绝不缓存登录态学习资源、题目、问答或案件数据；在线发现卡片撤回后会删除本地缓存。
- `GET /api/learning/resources` 和 `POST /api/knowledge/ask` 仅读取当前账号可见、已生效、已脱敏、已独立审核、已发布的培训或公开信息资源；`GET /api/learning/questions` 只返回培训用途的题目。
- 问答只返回匹配资源原文、来源、版本和人工核验提示；无可靠来源时返回 `insufficient_sources`，不生成行动建议。
- `POST /api/learning/questions/{id}/answers` 在服务端校验选项；题目列表和题目导出均不返回 `correct_option_id`。
- `GET /api/admin/learning/resources/{id}/export` 与 `GET /api/admin/learning/questions/{id}/export` 仅允许管理员获取当前已发布的受控版本，使用白名单响应字段并以 JSON 附件返回。

## 上线前人工核对

- 确认来源文件已获授权，且录入内容不含真实案件敏感信息。
- 由非提交人完成脱敏和审核，并写明理由。
- 核对生效时间、可见级别、用途和来源链接。
- 以学习账号、志愿者账号和家属账号分别验证可见范围。
- 撤回后复测资源列表、题库、问答、答题与导出，确认不再可访问。

本说明不等同于正式教材。正式防走失卡、手册和题目必须在上述流程中由具名负责人录入并审核后才会展示。
