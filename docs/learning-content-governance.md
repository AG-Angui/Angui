# 学习内容治理操作说明

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

资源提交额外要求来源名称、可选 HTTPS 来源地址、版本生效时间、可见级别和用途。`permitted_use` 目前仅允许 `training` 或 `public_information`；学习列表、问答、题库与导出只处理 `training` 内容，避免将公开信息、训练材料和其他用途混用。

## 读取与导出边界

- `GET /api/learning/resources`、`GET /api/learning/questions` 和 `POST /api/knowledge/ask` 仅读取当前账号可见、已生效、已脱敏、已独立审核、已发布的培训内容。
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
