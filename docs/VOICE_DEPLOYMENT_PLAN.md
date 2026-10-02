# 案件实时对讲与语音线索实施说明

## 拓扑与配置

浏览器通过 WSS 连接 LiveKit，由 LiveKit SFU 转发 WebRTC 音频；coturn 提供受限网络下的 TURN 中继。Angui API 校验案件与协作空间成员资格，签发短时房间令牌，并通过空间 WebSocket 同步协作事件。API 不传输实时音频。浏览器加入房间后录制实际 PCM WAV 片段，经受控上传接口写入私有音频卷。

### 媒体服务选型

本项目选用 LiveKit，不选 ZLMediaKit 作为对讲房间核心。LiveKit 原生提供多人 WebRTC 房间、SFU 转发、浏览器 SDK、短时房间令牌和发布权限，能够直接承载 PTT、指挥广播、断线重连及成员级发布/订阅控制。ZLMediaKit 更适合作为 RTSP/RTMP/WebRTC 协议网关和直播分发层；若用于本项目，需要另外实现房间信令、成员权限、发言仲裁、广播优先级和客户端状态同步。后续若需要摄像头直播或协议转换，可将 ZLMediaKit 作为独立媒体网关接入，不改变 LiveKit 对讲链路。

API 的后台 worker 从音频卷读取片段，调用私有 faster-whisper HTTP 服务转写，再使用已有 AI Gateway 生成待审核线索草稿。审核员确认前，不生成正式线索。preview 使用 CPU 小模型；生产可配置更大的模型与 GPU。生产需提供 LiveKit 外网域名、TLS、UDP/TURN 端口、AI Gateway 凭证和持久化音频卷。

## 接口与状态

- 房间令牌仅授予 active 空间的 active 指挥员或志愿者，短时过期；成员退出或空间归档时停止续签并断开。
- 普通成员一次只授予一位发言者；指挥广播可以打断。前端只有拿到服务端许可后才启用麦克风上行。
- 每个上传的 WAV 片段绑定空间、案件、提交人、时间范围与私有对象键。上传返回 queued；worker 依次转为 transcribing、transcribed、draft_ready，失败时记录原因并允许重试。
- 提交人确认或更正发现人；审核员可编辑候选、确认、驳回或退回。正式线索保留草稿到音频及转写的来源关联。

## 验证顺序

先验证 Compose 配置与服务健康，再用两个浏览器和三人房间验证 WebRTC、PTT、默认录音和权限撤销；使用真实 ASR 与 AI 提供方跑通语音到审核链。CI 使用受控 ASR/AI 测试服务验证成功、失败、重试与权限边界，不依赖外部凭证。
