import { useEffect, useRef, useState } from "react";
import { Mic, PhoneOff } from "lucide-react";
import {
  ConnectionState,
  LocalAudioTrack,
  Room,
  RoomEvent,
  Track,
  type RemoteTrack,
} from "livekit-client";
import {
  getVoiceRoomTicket,
  leaveVoiceRoom,
  setVoiceFloor,
  uploadIntercomRecording,
} from "../api/collaborationSpaces";
import { ApiClientError } from "../api/client";

const SEGMENT_MS = 20_000;

function wav(
  samples: Float32Array[],
  sampleRate: number,
  started: string,
): File {
  const length = samples.reduce((total, chunk) => total + chunk.length, 0);
  const buffer = new ArrayBuffer(44 + length * 2);
  const view = new DataView(buffer);
  const label = (offset: number, value: string) =>
    [...value].forEach((letter, index) =>
      view.setUint8(offset + index, letter.charCodeAt(0)),
    );
  label(0, "RIFF");
  view.setUint32(4, 36 + length * 2, true);
  label(8, "WAVE");
  label(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * 2, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  label(36, "data");
  view.setUint32(40, length * 2, true);
  let offset = 44;
  for (const chunk of samples)
    for (const sample of chunk) {
      const value = Math.max(-1, Math.min(1, sample));
      view.setInt16(offset, value < 0 ? value * 0x8000 : value * 0x7fff, true);
      offset += 2;
    }
  return new File([buffer], `room-${started.replace(/[:.]/g, "-")}.wav`, {
    type: "audio/wav",
  });
}

type Connection = "未加入" | "正在连接" | "已连接" | "重连中" | "已断开";

export function VoiceIntercomPanel({
  token,
  spaceId,
  onUploaded,
}: {
  token: string;
  spaceId: string;
  onUploaded: () => void;
}) {
  const [connection, setConnection] = useState<Connection>("未加入");
  const [recording, setRecording] = useState(false);
  const [speaking, setSpeaking] = useState(false);
  const [error, setError] = useState("");
  const [members, setMembers] = useState(0);
  const [participantNames, setParticipantNames] = useState<string[]>([]);
  const [microphones, setMicrophones] = useState<MediaDeviceInfo[]>([]);
  const [outputs, setOutputs] = useState<MediaDeviceInfo[]>([]);
  const [microphoneId, setMicrophoneId] = useState("");
  const [outputId, setOutputId] = useState("");
  const [volume, setVolume] = useState(1);
  const [pendingUploads, setPendingUploads] = useState(0);
  const outputIdRef = useRef("");
  const volumeRef = useRef(1);
  const roomRef = useRef<Room | null>(null);
  const joiningRef = useRef(false);
  const joinGenerationRef = useRef(0);
  const stoppingRef = useRef<Promise<void> | null>(null);
  const participantIdentityRef = useRef<string | null>(null);
  const floorGrantedRef = useRef(false);
  const pressPromiseRef = useRef<Promise<void> | null>(null);
  const releasePromiseRef = useRef<Promise<void> | null>(null);
  const microphoneRef = useRef<LocalAudioTrack | null>(null);
  const contextRef = useRef<AudioContext | null>(null);
  const workletRef = useRef<AudioWorkletNode | null>(null);
  const flushResolveRef = useRef<(() => void) | null>(null);
  const flushChainRef = useRef<Promise<void>>(Promise.resolve());
  const mixerRef = useRef<GainNode | null>(null);
  const ownGainRef = useRef<GainNode | null>(null);
  const sourcesRef = useRef(new Map<string, MediaStreamAudioSourceNode>());
  const audioElementsRef = useRef(new Map<string, HTMLAudioElement>());
  const chunksRef = useRef<Float32Array[]>([]);
  const startedRef = useRef("");
  const segmentTimerRef = useRef<number | null>(null);
  const floorTimerRef = useRef<number | null>(null);
  const pressedRef = useRef(false);
  const publishingRef = useRef(false);
  const uploadQueueRef = useRef<
    Array<{ file: File; start: string; end: string }>
  >([]);
  const uploadingRef = useRef(false);
  const drainUploads = async () => {
    if (uploadingRef.current) return;
    uploadingRef.current = true;
    try {
      while (uploadQueueRef.current.length > 0) {
        const item = uploadQueueRef.current[0];
        try {
          await uploadIntercomRecording(
            token,
            spaceId,
            item.file,
            item.start,
            item.end,
          );
          uploadQueueRef.current.shift();
          setPendingUploads(uploadQueueRef.current.length);
          onUploaded();
        } catch (cause) {
          setError(
            cause instanceof Error ? cause.message : "录音片段上传失败，请重试",
          );
          break;
        }
      }
    } finally {
      uploadingRef.current = false;
    }
  };
  const refreshDevices = () => {
    void navigator.mediaDevices
      ?.enumerateDevices()
      .then((devices) => {
        setMicrophones(
          devices.filter((device) => device.kind === "audioinput"),
        );
        setOutputs(devices.filter((device) => device.kind === "audiooutput"));
      })
      .catch(() => undefined);
  };

  const flushOnce = async () => {
    if (workletRef.current) {
      await new Promise<void>((resolve) => {
        const timeout = window.setTimeout(() => {
          flushResolveRef.current = null;
          resolve();
        }, 1000);
        flushResolveRef.current = () => {
          window.clearTimeout(timeout);
          resolve();
        };
        workletRef.current?.port.postMessage({ type: "flush" });
      });
    }
    const context = contextRef.current;
    const chunks = chunksRef.current;
    const start = startedRef.current;
    chunksRef.current = [];
    startedRef.current = new Date().toISOString();
    if (!context || !chunks.length || !start) return;
    const end = new Date().toISOString();
    const file = wav(chunks, context.sampleRate, start);
    uploadQueueRef.current.push({ file, start, end });
    setPendingUploads(uploadQueueRef.current.length);
    void drainUploads();
    if (uploadQueueRef.current.length >= 10) {
      setError("录音片段待上传过多，已退出房间；请恢复网络后重试上传。");
      roomRef.current?.disconnect();
    }
  };
  const flush = () => {
    const next = flushChainRef.current.then(flushOnce);
    flushChainRef.current = next.catch(() => setError("无法结束录音片段"));
    return next;
  };

  const releaseFloor = async () => {
    pressedRef.current = false;
    if (floorTimerRef.current !== null)
      window.clearInterval(floorTimerRef.current);
    floorTimerRef.current = null;
    if (ownGainRef.current) ownGainRef.current.gain.value = 0;
    const room = roomRef.current;
    const microphone = microphoneRef.current;
    if (room && microphone) {
      try {
        if (
          room.state === ConnectionState.Connected &&
          Array.from(room.localParticipant.trackPublications.values()).some(
            (publication) => publication.track === microphone,
          )
        )
          await room.localParticipant.unpublishTrack(microphone, false);
      } catch {
        /* disconnected */
      }
      microphone.mediaStreamTrack.enabled = false;
    }
    const identity = participantIdentityRef.current;
    if (identity && floorGrantedRef.current) {
      try {
        await setVoiceFloor(token, spaceId, identity, false);
      } catch {
        /* A disconnected participant is removed by LiveKit. */
      }
      floorGrantedRef.current = false;
    }
    setSpeaking(false);
  };
  const release = () => {
    if (releasePromiseRef.current) return releasePromiseRef.current;
    const pending = releaseFloor();
    releasePromiseRef.current = pending;
    void pending.finally(() => {
      if (releasePromiseRef.current === pending) releasePromiseRef.current = null;
    });
    return pending;
  };

  const stop = () => {
    if (stoppingRef.current) return stoppingRef.current;
    joinGenerationRef.current += 1;
    const pending = (async () => {
      pressedRef.current = false;
      if (pressPromiseRef.current) await pressPromiseRef.current;
      await release();
      if (segmentTimerRef.current !== null)
        window.clearInterval(segmentTimerRef.current);
      segmentTimerRef.current = null;
      try {
        await flush();
      } catch {
        setError("无法结束录音片段");
      }
      setRecording(false);
      const currentRoom = roomRef.current;
      roomRef.current = null;
      await currentRoom?.disconnect();
      const identity = participantIdentityRef.current;
      participantIdentityRef.current = null;
      if (identity) {
        try {
          await leaveVoiceRoom(token, spaceId, identity);
        } catch {
          /* The server will revoke the room session when membership expires. */
        }
      }
      microphoneRef.current?.stop();
      microphoneRef.current = null;
      for (const audio of audioElementsRef.current.values()) audio.remove();
      audioElementsRef.current.clear();
      sourcesRef.current.clear();
      void contextRef.current?.close();
      contextRef.current = null;
      workletRef.current = null;
      mixerRef.current = null;
      ownGainRef.current = null;
      setMembers(0);
      setParticipantNames([]);
      setConnection("已断开");
    })();
    stoppingRef.current = pending;
    void pending.finally(() => {
      stoppingRef.current = null;
    });
    return pending;
  };

  const join = async () => {
    if (joiningRef.current || roomRef.current || stoppingRef.current) return;
    joiningRef.current = true;
    const generation = ++joinGenerationRef.current;
    setConnection("正在连接");
    setError("");
    let media: MediaStream | null = null;
    let context: AudioContext | null = null;
    let room: Room | null = null;
    let identity: string | null = null;
    try {
      const ticket = await getVoiceRoomTicket(token, spaceId);
      identity = ticket.participant_identity;
      if (generation !== joinGenerationRef.current) return;
      participantIdentityRef.current = identity;
      media = await navigator.mediaDevices.getUserMedia({
        audio: {
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
          ...(microphoneId ? { deviceId: { ideal: microphoneId } } : {}),
        },
      });
      if (generation !== joinGenerationRef.current) return;
      refreshDevices();
      const rawTrack = media.getAudioTracks()[0];
      rawTrack.enabled = false;
      const microphone = new LocalAudioTrack(rawTrack, undefined, true);
      context = new AudioContext();
      await context.audioWorklet.addModule("/audio-recorder-worklet.js");
      await context.resume();
      const mixer = context.createGain();
      const worklet = new AudioWorkletNode(context, "pcm-recorder");
      const mute = context.createGain();
      mute.gain.value = 0;
      mixer.connect(worklet);
      worklet.connect(mute);
      mute.connect(context.destination);
      worklet.port.onmessage = (
        event: MessageEvent<Float32Array | { type: string }>,
      ) => {
        if (event.data instanceof Float32Array)
          chunksRef.current.push(event.data);
        else if (event.data.type === "flushed") {
          flushResolveRef.current?.();
          flushResolveRef.current = null;
        }
      };
      const ownSource = context.createMediaStreamSource(
        new MediaStream([rawTrack]),
      );
      const ownGain = context.createGain();
      ownGain.gain.value = 0;
      ownSource.connect(ownGain);
      ownGain.connect(mixer);
      room = new Room({ adaptiveStream: true, dynacast: true });
      const attach = (track: RemoteTrack) => {
        if (track.kind !== Track.Kind.Audio || !context || !mixer) return;
        const audio = track.attach() as HTMLAudioElement;
        audio.autoplay = true;
        audio.volume = volumeRef.current;
        audio.style.display = "none";
        if (outputIdRef.current && "setSinkId" in audio) {
          void audio
            .setSinkId(outputIdRef.current)
            .catch(() => setError("音频输出设备不可用"));
        }
        document.body.appendChild(audio);
        void audio
          .play()
          .catch(() => setError("浏览器阻止了音频播放，请点击页面后重试。"));
        audioElementsRef.current.set(track.sid || "", audio);
        const source = context.createMediaStreamSource(
          new MediaStream([track.mediaStreamTrack]),
        );
        source.connect(mixer);
        sourcesRef.current.set(track.sid || "", source);
      };
      room.on(RoomEvent.TrackSubscribed, (track) => attach(track));
      room.on(RoomEvent.TrackUnsubscribed, (track) => {
        sourcesRef.current.get(track.sid || "")?.disconnect();
        sourcesRef.current.delete(track.sid || "");
        audioElementsRef.current.get(track.sid || "")?.remove();
        audioElementsRef.current.delete(track.sid || "");
      });
      const syncParticipants = () => {
        setMembers(room!.remoteParticipants.size + 1);
        setParticipantNames([
          room!.localParticipant.name || "我",
          ...Array.from(room!.remoteParticipants.values()).map(
            (member) => member.name || member.identity,
          ),
        ]);
      };
      room.on(RoomEvent.ParticipantConnected, syncParticipants);
      room.on(RoomEvent.ParticipantDisconnected, syncParticipants);
      room.on(RoomEvent.Reconnecting, () => {
        pressedRef.current = false;
        microphone.mediaStreamTrack.enabled = false;
        if (floorTimerRef.current !== null)
          window.clearInterval(floorTimerRef.current);
        floorTimerRef.current = null;
        setSpeaking(false);
        setConnection("重连中");
      });
      room.on(RoomEvent.Reconnected, () => {
        void release().then(() => setConnection("已连接"));
      });
      room.on(RoomEvent.Disconnected, (reason) => {
        if (roomRef.current === room && !stoppingRef.current) {
          setError(`语音连接已断开（原因代码 ${reason ?? "未知"}），请检查网络后重新加入`);
          void stop();
        }
      });
      await room.connect(ticket.url, ticket.token, {
        autoSubscribe: true,
        rtcConfig: ticket.ice_servers.length
          ? { iceServers: ticket.ice_servers }
          : undefined,
      });
      if (generation !== joinGenerationRef.current) return;
      roomRef.current = room;
      microphoneRef.current = microphone;
      contextRef.current = context;
      workletRef.current = worklet;
      mixerRef.current = mixer;
      ownGainRef.current = ownGain;
      chunksRef.current = [];
      startedRef.current = new Date().toISOString();
      segmentTimerRef.current = window.setInterval(() => {
        void flush();
      }, SEGMENT_MS);
      setMembers(room.remoteParticipants.size + 1);
      syncParticipants();
      setRecording(true);
      setConnection("已连接");
    } catch (cause) {
      if (generation === joinGenerationRef.current) {
        setConnection("未加入");
        setError(
          cause instanceof ApiClientError && cause.detail
            ? cause.detail
            : cause instanceof Error
              ? cause.message
              : "无法加入语音房间",
        );
      }
    } finally {
      if (roomRef.current !== room) {
        await room?.disconnect();
        media?.getTracks().forEach((track) => track.stop());
        void context?.close();
        if (identity) {
          if (participantIdentityRef.current === identity)
            participantIdentityRef.current = null;
          void leaveVoiceRoom(token, spaceId, identity).catch(() => undefined);
        }
      }
      joiningRef.current = false;
    }
  };

  const startPress = async () => {
    if (
      pressedRef.current ||
      !roomRef.current ||
      !microphoneRef.current ||
      publishingRef.current ||
      releasePromiseRef.current ||
      stoppingRef.current ||
      roomRef.current.state !== ConnectionState.Connected ||
      !participantIdentityRef.current
    )
      return;
    pressedRef.current = true;
    publishingRef.current = true;
    const identity = participantIdentityRef.current;
    try {
      await setVoiceFloor(token, spaceId, identity, true);
      floorGrantedRef.current = true;
      if (!pressedRef.current) {
        await release();
        return;
      }
      for (let attempt = 0;
        attempt < 40 && roomRef.current && !roomRef.current.localParticipant.permissions?.canPublish;
        attempt += 1) {
        await new Promise((resolve) => window.setTimeout(resolve, 75));
      }
      if (!roomRef.current?.localParticipant.permissions?.canPublish)
        throw new Error("发言许可未同步，请重试");
      const microphone = microphoneRef.current;
      const room = roomRef.current;
      if (
        !microphone || !room || !pressedRef.current ||
        room.state !== ConnectionState.Connected
      ) {
        await release();
        return;
      }
      microphone.mediaStreamTrack.enabled = true;
      await room.localParticipant.publishTrack(microphone, {
        audioPreset: { maxBitrate: 32000 },
      });
      if (!pressedRef.current) {
        await release();
        return;
      }
      if (ownGainRef.current) ownGainRef.current.gain.value = 1;
      floorTimerRef.current = window.setInterval(() => {
        void setVoiceFloor(token, spaceId, identity, true).catch(() => {
          void release();
          setError("发言许可已失效");
        });
      }, 5_000);
      setSpeaking(true);
    } catch (cause) {
      pressedRef.current = false;
      if (microphoneRef.current)
        microphoneRef.current.mediaStreamTrack.enabled = false;
      await release();
      setError(
        cause instanceof ApiClientError && cause.detail
          ? cause.detail
          : cause instanceof Error
            ? cause.message
            : "当前无法发言",
      );
    } finally {
      publishingRef.current = false;
    }
  };

  const press = () => {
    if (pressPromiseRef.current) return;
    const pending = startPress();
    pressPromiseRef.current = pending;
    void pending.finally(() => {
      if (pressPromiseRef.current === pending) pressPromiseRef.current = null;
    });
  };

  const stopRef = useRef(stop);
  stopRef.current = stop;
  useEffect(() => {
    const stopLatest = () => stopRef.current();
    return () => {
      if (segmentTimerRef.current !== null)
        window.clearInterval(segmentTimerRef.current);
      if (floorTimerRef.current !== null)
        window.clearInterval(floorTimerRef.current);
      void stopLatest();
    };
  }, []);
  useEffect(refreshDevices, []);

  return (
    <div
      className="mt-3 border-t border-slate-200 pt-3"
      aria-label="实时语音对讲"
    >
      <div className="flex items-center justify-between gap-2">
        <strong className="flex items-center gap-1 text-xs text-slate-800">
          <Mic size={14} />
          实时语音对讲
        </strong>
        <span className="text-xs text-red-700" role="status">
          {recording ? "正在录音" : "未录音"} · {connection} · {members} 人
        </span>
      </div>
      <p className="mt-1 text-xs text-slate-600">
        加入房间即开始私有录音并生成待审核线索；房间成员均会看到录音提示。
      </p>
      {recording && (
        <p className="mt-1 text-xs text-red-700">
          房间成员录音中：{participantNames.join("、")}
        </p>
      )}
      <div className="mt-2 flex flex-wrap items-center gap-2 text-xs">
        <label>
          麦克风{" "}
          <select
            aria-label="选择麦克风"
            className="border p-1"
            disabled={recording}
            value={microphoneId}
            onChange={(event) => setMicrophoneId(event.target.value)}
          >
            <option value="">默认设备</option>
            {microphones
              .filter((device) => device.deviceId)
              .map((device, index) => (
                <option key={device.deviceId} value={device.deviceId}>
                  {device.label || `麦克风 ${index + 1}`}
                </option>
              ))}
          </select>
        </label>
        {outputs.length > 0 && (
          <label>
            扬声器{" "}
            <select
              aria-label="选择扬声器"
              className="border p-1"
              value={outputId}
              onChange={(event) => {
                const next = event.target.value;
                setOutputId(next);
                outputIdRef.current = next;
                for (const audio of audioElementsRef.current.values())
                  if ("setSinkId" in audio)
                    void audio
                      .setSinkId(next)
                      .catch(() => setError("音频输出设备不可用"));
              }}
            >
              <option value="">默认设备</option>
              {outputs
                .filter((device) => device.deviceId)
                .map((device, index) => (
                  <option key={device.deviceId} value={device.deviceId}>
                    {device.label || `扬声器 ${index + 1}`}
                  </option>
                ))}
            </select>
          </label>
        )}
        <label>
          音量{" "}
          <input
            aria-label="接收音量"
            max="1"
            min="0"
            step="0.1"
            type="range"
            value={volume}
            onChange={(event) => {
              const next = Number(event.target.value);
              setVolume(next);
              volumeRef.current = next;
              for (const audio of audioElementsRef.current.values())
                audio.volume = next;
            }}
          />
        </label>
      </div>
      {connection !== "已连接" && connection !== "重连中" ? (
        <button
          className="mt-2 rounded bg-slate-800 px-3 py-1 text-sm text-white"
          disabled={connection === "正在连接"}
          onClick={() => void join()}
          type="button"
        >
          加入对讲房间
        </button>
      ) : (
        <div className="mt-2 flex gap-2">
          <button
            aria-label="按住发言"
            className="rounded bg-blue-700 px-3 py-1 text-sm text-white disabled:opacity-50"
            disabled={connection !== "已连接"}
            onPointerDown={() => void press()}
            onPointerUp={() => void release()}
            onPointerCancel={() => void release()}
            onPointerLeave={() => {
              if (pressedRef.current) void release();
            }}
            onKeyDown={(event) => {
              if (
                (event.key === " " || event.key === "Enter") &&
                !event.repeat
              ) {
                event.preventDefault();
                void press();
              }
            }}
            onKeyUp={(event) => {
              if (event.key === " " || event.key === "Enter") {
                event.preventDefault();
                void release();
              }
            }}
            type="button"
          >
            {speaking ? "正在发言" : "按住发言"}
          </button>
          <button
            className="flex items-center gap-1 rounded border px-3 py-1 text-sm"
            onClick={() => void stop()}
            type="button"
          >
            <PhoneOff size={14} />
            退出
          </button>
        </div>
      )}
      {error && (
        <p className="mt-2 text-xs text-red-700" role="alert">
          {error}
        </p>
      )}
      {pendingUploads > 0 && (
        <button
          className="mt-2 text-xs underline"
          type="button"
          onClick={() => void drainUploads()}
        >
          重试上传 {pendingUploads} 个录音片段
        </button>
      )}
    </div>
  );
}
