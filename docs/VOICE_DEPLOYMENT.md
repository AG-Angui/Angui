# 实时对讲及语音线索部署

部署启动 LiveKit、coturn、faster-whisper 和 Angui API。浏览器向 API 申请五分钟 LiveKit 房间令牌；音频经 WebRTC 传输，协作事件经一次性票据认证的 WebSocket 传输。入房即录音，每 20 秒上传一段 PCM WAV 到私有音频卷。API 的队列 worker 调用 ASR 和现有 AI Gateway，生成待审核草稿；提交人确认发现人后，指挥员才能接受草稿。接受草稿生成的正式线索仍遵循既有线索审核流程。

LiveKit 是本项目的实时对讲核心：它负责多人 WebRTC/SFU、房间令牌和发布权限。ZLMediaKit 不在当前对讲部署中；它适合后续独立承担 RTSP/RTMP/WebRTC 协议转换或直播分发，不能替代本方案中的房间权限和 PTT 仲裁服务。

Compose 固定使用 LiveKit `v1.13.7`，与前端 `livekit-client 2.22.3` 配套。旧版 `v1.9.7` 对 SDK 首次访问的 `/rtc/v1` 返回 404，SDK 会回退到 `/rtc`；此回退本身不证明后续断线原因。房间令牌现在为每次入房签发独立的参与者身份，同一账号多个窗口不会因身份重复被 LiveKit 挤出。退出会释放发言权并移除该窗口的参与者。

## 生产环境

从仓库目录使用 `deploy/production/compose.yml`。除现有 `API_IMAGE`、`WEB_IMAGE`、`DATABASE_URL`、`FRONTEND_ORIGIN` 外，配置：

| 变量 | 用途 |
| --- | --- |
| `LIVEKIT_PUBLIC_URL` | 浏览器可达的 `wss://` LiveKit 域名；反向代理转发到本机 `LIVEKIT_HTTP_PORT` |
| `LIVEKIT_API_KEY` / `LIVEKIT_API_SECRET` | LiveKit 签名密钥，secret 至少 32 字符 |
| `TURN_PUBLIC_URL` / `TURN_TLS_PUBLIC_URL` / `TURN_SECRET` / `TURN_REALM` / `PUBLIC_IP` | coturn UDP/TLS 公网地址、共享鉴权密钥、域名和服务器公网 IP |
| `TURN_CERT_FILE` / `TURN_KEY_FILE` | 与 TURN TLS 域名匹配的证书和私钥绝对路径 |
| `ANGUI_ASR_KEY` / `ASR_MODEL` | API 与 ASR 容器共用的私有令牌及模型；默认 medium/CPU |
| `ANGUI_AI_PROVIDERS_JSON` / `ANGUI_VOICE_AI_ENDPOINT` / `ANGUI_VOICE_AI_KEY` | 现有 AI Gateway 的结构化提取策略、供应商地址与凭证 |

AI 策略必须允许 `structured_extraction`、`collaborative`、`clue_draft` 和 `CN`。策略的 `endpoint_env` 指向 `ANGUI_VOICE_AI_ENDPOINT`，`credential_env` 指向 `ANGUI_VOICE_AI_KEY`。请按 [AI Gateway 配置](AI_GATEWAY.md)核对完整 JSON。没有合规 AI 供应商时，ASR 转写可以完成，但候选会明确失败并保留音频供重试。

反向代理需为 LiveKit 提供 TLS/WSS 和 WebSocket Upgrade，转发到本机 TCP 7880；浏览器应用和 LiveKit 域名均须使用有效证书。开放 LiveKit UDP 7882、TCP 7881、coturn UDP 3478、TLS/TCP 5349 与 UDP 49160–49200（端口均可通过 Compose 变量修改）。coturn 通过短期签名凭据鉴权，API 每次入房签发 10 分钟 ICE 凭据。生产同时下发 UDP 与 TLS TURN 地址，需在上线前验证弱网质量。

音频卷 `audio-data` 与 ASR 模型卷 `asr-models` 必须持久化；数据库迁移先于新 API 启动。ASR 首次启动会下载模型，模型下载与资源占用由部署方监控。若改用 GPU，设置 `ASR_DEVICE=cuda`、相应 `ASR_COMPUTE_TYPE`，并为容器提供 CUDA 运行时。

## Preview

预览部署脚本为每个预览分配独立的 LiveKit/TURN 端口和短期密钥。`livekit-<预览域名>` 必须解析到承载媒体 UDP 端口的单个源站公网 IPv4；脚本会从该域名自动取得 coturn 宣告的中继 IP，无需设置 `PREVIEW_PUBLIC_IP`。如果域名经过 CDN 代理、DNS 返回的是网关地址，或源站地址与解析结果不同，请在 GitHub Actions 变量中显式设置 `PREVIEW_PUBLIC_IP` 为媒体源站公网 IP。普通 HTTPS 反向代理不能转发 WebRTC/TURN UDP。预览 ASR 默认 small/CPU，模型容器镜像首次部署时构建，模型缓存卷在预览数据库重置后保留。已有 `ANGUI_AI_PROVIDERS_JSON`、`ANGUI_PREVIEW_AI_ENDPOINT`、`ANGUI_PREVIEW_AI_KEY` 配置可供语音结构化提取复用。

PR/分支预览必须使用通过 CI 的同一提交中的 `deploy/preview` 文件；旧版 Compose 不会向新后端传入 LiveKit 配置，申请 `/voice-room/ticket` 会返回 `409 LiveKit is not configured`，旧版预览 Nginx 也没有协作空间 WebSocket 的 Upgrade 规则。`workflow_run` 从默认分支读取 workflow，因此预览 workflow 的修复必须先进入默认分支，之后触发的预览才会采用。若公网入口还有一层 Nginx，应用域名的 `/api/collaboration-spaces/` 和 LiveKit 域名都需要以 HTTP/1.1 转发 `Upgrade`、`Connection` 头；新版内部预览 Nginx 已配置，外层代理也必须配置。用有效的 `/events/ticket` 一次性票据验证 `/events/ws` 返回 `101 Switching Protocols`；若仍返回 `400`，检查各层代理的 WebSocket 握手头。WebSocket 断线后页面仍会通过 HTTP 补偿协作事件，但实时连接必须单独验收。

部署后先检查 API、LiveKit、ASR 容器健康，再用两个浏览器确认入房、PTT 和录音上传；三人房间和 TURN 网络分别验证。权限撤销、失败重试及人工审核应在真实服务联调中验证，不以合成音频替代媒体验收。

若连接后仍突然断开，先记录浏览器页面显示的 LiveKit 断开原因代码和断开时间，再核对 LiveKit 日志、该预览的 `PREVIEW_LIVEKIT_UDP_PORT`、`PREVIEW_LIVEKIT_TCP_PORT` 及 coturn 端口是否从浏览器网络可达。信令 WSS 成功只代表控制连接成功，不代表 UDP 媒体路径已连通。预览更换 Compose 镜像后需重新部署对应预览，旧容器不会自动升级。
