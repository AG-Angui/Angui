# 安归前端

React + TypeScript + Vite 前端，位于仓库的 `frontend/` 目录。组件与可访问交互使用 HeroUI 3，布局和响应式样式使用 Tailwind CSS 4。当前已经接入会话认证、角色导航、案件列表/创建/详情、案件成员邀请、线索提交/审核和案件状态流转。

## UI 基线

- HeroUI：卡片、状态标签、按钮、输入和加载状态。
- Tailwind CSS：页面布局、响应式断点、间距、颜色和排版。
- Lucide React：导航、指标和空状态图标。
- React Router：总览、家属端、指挥端和志愿者端路由。
- 品牌标识：直接复用仓库根目录 `assets/brand/angui-mark.svg`。

前端隐藏按钮不构成权限控制。所有案件关系、线索可见性、健康字段裁剪和操作权限都由 Actix 服务端重新验证。Bearer 令牌只保存在当前标签页的 `sessionStorage`；正式部署必须使用 HTTPS。

## 运行

从仓库根目录执行：

```powershell
npm install --prefix frontend
npm run dev:frontend
npm run lint:frontend
npm run build:frontend
```

开发地址为 `http://127.0.0.1:5173`。Vite 会把 `/api` 请求代理到默认后端地址 `http://127.0.0.1:8080`。

## 检查

```powershell
npm run lint:frontend
npm run build:frontend
npm --prefix frontend run test
```

### 前端测试基线

测试使用 Vitest、Testing Library、`.invalid` 模拟邮箱和虚构案件数据，不调用真实服务，也不在断言中记录密码或会话令牌。

| 能力 / 后端端点                                                                          | 前端测试                        |
| ---------------------------------------------------------------------------------------- | ------------------------------- |
| API 网络错误，以及 `400`、`401`、`403`、`404`、`409` 错误的安全提示                      | `src/api/client.test.ts`        |
| `GET/POST /api/cases`、`GET /api/cases/{case_id}`、成员添加、线索提交/审核、案件状态更新 | `src/api/cases.test.ts`         |
| 登录成功/失败、刷新恢复、退出、令牌失效后清除旧会话                                      | `src/auth/AuthContext.test.tsx` |
| 无会话页面、家属/指挥/志愿者导航、错误工作台 URL 重定向、新人和管理员的无案件权限状态    | `src/App.test.tsx`              |

运行一次完整前端基线：

```powershell
npm --prefix frontend run test
```

## 环境变量

复制 `frontend/.env.example` 为本地环境文件时，不要提交真实服务地址或密钥。

| 变量                | 默认值 | 用途                  |
| ------------------- | ------ | --------------------- |
| `VITE_API_BASE_URL` | `/api` | 浏览器调用的 API 前缀 |
| `VITE_AMAP_JS_API_KEY` | 空 | 浏览器端 AMap JSAPI key；仅用于用户主动触发的位置确认地图，须按预览/生产域名限制，不能使用服务端 `AMAP_WEBSERVICE_KEY`。 |
| `VITE_AMAP_JS_API_SERVICE_HOST` | `/_AMapService` | AMap JSAPI v2 的同源安全代理地址。生产环境不要改为 AMap 直连地址。 |
| `VITE_AMAP_JS_API_SECURITY_CODE` | 空 | 仅限本地开发直连时使用；生产构建必须为空，生产安全码由 Nginx 容器运行时注入。 |
