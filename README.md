# 安归

> 面向失智老人走失搜救的 AI + 地图协同系统。

安归希望帮助家属、搜救指挥人员和志愿者在老人走失事件中共享一套经过审核的信息：AI 负责辅助问询、整理线索和生成摘要，地图负责呈现点位、任务、路线与安全态势。系统的目标是减少信息遗漏和重复沟通，同时保护老人、家属与志愿者的隐私和人身安全。

## 当前状态

本仓库目前处于 **MVP 纵向闭环开发阶段**。React 工作台、Rust/Actix Web 后端、SeaORM 数据层、可撤销会话认证、案件级角色授权、版本化迁移，以及案件创建、查询、成员授权、线索提交、人工审核和状态流转已经落地。任务、地图、AI、密码找回、多因素认证和生产部署仍属于后续实现范围；当前实现不能视为已经部署或经过真实搜救验证的产品。

- 需求整理入口：[docs/PRODUCT.md](./docs/PRODUCT.md)
- 文档入口：[docs/README.md](./docs/README.md)
- 产品范围：[docs/PRODUCT.md](./docs/PRODUCT.md)
- 技术架构草案：[docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)
- API 说明：[docs/API.md](./docs/API.md)
- 数据库与迁移规范：[docs/DATABASE.md](./docs/DATABASE.md)
- 数据与 AI 规范：[docs/DATA_AND_AI.md](./docs/DATA_AND_AI.md)
- 安全与隐私边界：[docs/SECURITY_AND_PRIVACY.md](./docs/SECURITY_AND_PRIVACY.md)
- Demo 与初赛交付：[docs/DEMO_AND_DELIVERY.md](./docs/DEMO_AND_DELIVERY.md)
- 贡献指南：[CONTRIBUTING.md](./CONTRIBUTING.md)
- 行为准则：[CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- 分发规范：[DISTRIBUTION.md](./DISTRIBUTION.md)
- Codex 协作说明：[CODEX.md](./CODEX.md)
- CI、AI 评审与分支预览配置：[.github/WORKFLOWS.md](./.github/WORKFLOWS.md)
- 品牌图标与仓库图像：[assets/brand/README.md](./assets/brand/README.md)

## MVP 场景

初版计划通过一例完全脱敏的模拟案件演示以下闭环：

1. 家属端通过引导式问询提交老人基本情况、最后出现地点和常去地点。
2. AI 生成待人工确认的老人画像与线索摘要。
3. 指挥端建立案件，审核线索并生成公共实时案情。
4. 地图展示最后出现点、线索点、任务点、重点 POI 和模拟轨迹。
5. 指挥人员向志愿者分配任务；志愿者导航、上报位置并反馈线索。
6. 指挥人员审核新线索，更新案情和任务状态。
7. 案件结束后，对聊天、任务和轨迹材料进行脱敏、复盘并归档为案例。

## 计划中的角色与端

| 角色/端 | 主要职责 | 信息边界 |
| --- | --- | --- |
| 家属端 | 提交信息、补充线索、查看经确认的公开进展 | 不可查看内部调度、未核实线索或志愿者位置 |
| 指挥端 | 审核线索、维护案情、分配任务、查看安全态势 | 敏感操作必须鉴权、确认并留痕 |
| 志愿者端 | 查看本人任务、导航、位置上报、线索反馈 | 默认不展示全量案件或他人完整轨迹 |
| 学习中心 | 提供手册、案例、防走失知识和理论题库 | 新人不可进入真实案件操作区 |
| 管理后台 | 权限、配置、审计与案例脱敏 | 技术人员原则上不接触明文隐私 |

## 技术架构

- 前端：React 19 + TypeScript + Vite + HeroUI 3 + Tailwind CSS 4，位于 `frontend/`。
- 后端：Rust 2024 + Actix Web，位于仓库根目录的 `src/`。
- 当前接口：健康检查、登录/登出/当前用户，以及案件创建/列表/详情、成员授权、案件状态流转、线索提交和线索人工审核接口。
- 数据：SeaORM 1.1 稳定版本线，编译支持 SQLite、PostgreSQL 和 MySQL；结构变更通过 `sea-orm-migration` 和三套同编号 SQL 脚本管理。
- 认证：Argon2id 密码哈希、数据库会话、SHA-256 令牌哈希、8 小时默认有效期和服务端撤销；案件访问由 `case_memberships` 强制约束。
- AI：通义千问/百炼 Agent 与 RAG，用于辅助问询、结构化、摘要、知识问答和案例整理。
- 地图：高德地图 JS API/Web 服务 API，用于点位、POI、路线和轨迹展示。

React、Rust/Actix Web、SeaORM 数据持久化和首批身份权限已经进入仓库。当前只在 SQLite 上完成实际迁移与认证/RBAC HTTP 联调；PostgreSQL/MySQL 已提供驱动、方言 SQL 和 CI 服务容器验证配置。AI、地图、任务和生产级账号生命周期仍是计划能力，在代码、配置、测试和验证结果进入仓库前，不应在介绍材料中宣称已经完成。

## 不可突破的安全边界

- 不接入未经授权的官方或社会监控资源，不绕过任何授权流程。
- 不实现人脸识别或跨摄像头人员确认。
- AI 不得把推断包装为事实，不输出“老人一定在哪里”；只可给出带依据和不确定性说明的待核实建议。
- 未经指挥人员确认，不得把 AI 结果自动转成正式线索、公共通报或现场任务。
- 不自动派遣志愿者进入山林、水域、铁路、高速道路、施工区等高风险区域。
- 不向家属、公众或无关参与者展示志愿者实时位置、完整身份信息、病史、联系方式或原始聊天记录。
- 演示、测试和公开材料默认使用虚构或充分脱敏的数据。

更完整的要求见 [安全与隐私文档](./docs/SECURITY_AND_PRIVACY.md)。

## 本地开发

环境要求：

- Node.js 24 或项目依赖支持的活跃 LTS 版本；
- npm 11；
- Rust 1.97+ 与 Cargo。

安装前端依赖：

```powershell
npm install --prefix frontend
```

创建本地数据库结构。应用启动时不会自动建表，必须显式执行迁移：

```powershell
$env:DATABASE_URL = "sqlite://data/angui.db?mode=rwc"
npm run migrate:up
npm run migrate:status
```

显式创建固定的低权限模拟账号。密码只通过当前终端环境变量提供，不写入仓库：

```powershell
$env:ANGUI_DEMO_PASSWORD = "replace-with-at-least-12-characters"
npm run auth:bootstrap-demo
```

命令创建或更新以下保留域账号，并撤销它们之前的会话：

```text
family@demo.invalid
commander@demo.invalid
volunteer@demo.invalid
```

分别启动两个开发进程：

```powershell
npm run dev:backend
npm run dev:frontend
```

- 前端：`http://127.0.0.1:5173`
- 后端健康检查：`http://127.0.0.1:8080/api/health`

质量检查：

```powershell
npm run format:backend
npm run check:backend
npm run test:backend
npm run lint:frontend
npm run build:frontend
```

数据库迁移命令：

```powershell
npm run migrate:up
npm run migrate:down
npm run migrate:status
```

这些命令都读取 `DATABASE_URL`。后端配置示例见 `.env.example`，前端配置示例见 `frontend/.env.example`。程序读取系统环境变量，不会自动加载 `.env` 文件。

前端令牌保存在浏览器当前标签页的 `sessionStorage`，服务端只保存令牌哈希。正式部署必须使用 HTTPS，并补充密码重置、多因素认证、持久化/分布式登录限流、账号审批和安全运营流程。演示账号只能处理虚构或充分脱敏的数据。

## 自动化与分支预览

GitHub Actions 已配置 Rust/前端质量门禁、PostgreSQL/MySQL 迁移检查、可选 OpenAI-compatible 自定义 API 评审，以及可选 GHCR 分支镜像和远程预览。仓库不再托管独立 Gemini workflow；如需 Gemini，应按当前官方企业集成方式配置。镜像构建与部署默认关闭，只有设置对应仓库变量后才会运行；远程部署还需要明确配置 SSH/GHCR 凭据。详细变量、密钥、配额和自托管 runner 边界见 [.github/WORKFLOWS.md](./.github/WORKFLOWS.md)。

预览容器会先显式运行 `migration up`，再通过 `angui-admin bootstrap-demo` 初始化 `.invalid` 模拟账号，全部成功后才启动 API；应用本身仍不会自动修改数据库结构或创建账号。

任何真实密钥、真实案件数据和个人信息都不得提交到仓库。

## 参与项目

提交更改前请阅读 [CONTRIBUTING.md](./CONTRIBUTING.md) 和 [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)。涉及权限、定位、隐私、AI 决策或真实搜救流程的改动属于高风险改动，需要专项评审和安全验证。

## 许可与分发

仓库当前尚未提供开源许可证，因此默认保留全部权利，不应擅自复制、公开发布、二次分发或用于真实搜救。演示包、测试包和未来正式版本的分发规则见 [DISTRIBUTION.md](./DISTRIBUTION.md)。第三方服务还需分别遵守通义千问/阿里云、高德地图及其他依赖的服务条款和许可。

## 重要声明

安归是搜救信息协同工具的设计方案，不是警方、消防、医疗或专业救援机构的替代品。发生真实走失事件时，应优先报警并联系当地具备资质的救援力量；任何系统建议都必须由具备权限和现场经验的人员复核。
