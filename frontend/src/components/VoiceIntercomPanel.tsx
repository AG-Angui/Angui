import { useRef, useState } from "react";
import { Button } from "@heroui/react";
import { Mic, Square } from "lucide-react";
import { uploadVoiceReport } from "../api/collaborationSpaces";

function encodeWav(samples: Float32Array[], sampleRate: number) {
  const length = samples.reduce((total, chunk) => total + chunk.length, 0);
  const buffer = new ArrayBuffer(44 + length * 2);
  const view = new DataView(buffer);
  const write = (offset: number, value: string) => [...value].forEach((character, index) => view.setUint8(offset + index, character.charCodeAt(0)));
  write(0, "RIFF"); view.setUint32(4, 36 + length * 2, true); write(8, "WAVE"); write(12, "fmt "); view.setUint32(16, 16, true); view.setUint16(20, 1, true); view.setUint16(22, 1, true); view.setUint32(24, sampleRate, true); view.setUint32(28, sampleRate * 2, true); view.setUint16(32, 2, true); view.setUint16(34, 16, true); write(36, "data"); view.setUint32(40, length * 2, true);
  let offset = 44;
  for (const chunk of samples) for (const sample of chunk) { const value = Math.max(-1, Math.min(1, sample)); view.setInt16(offset, value < 0 ? value * 0x8000 : value * 0x7fff, true); offset += 2; }
  return new File([buffer], `intercom-${new Date().toISOString().replace(/[:.]/g, "-")}.wav`, { type: "audio/wav" });
}

export function VoiceIntercomPanel({ token, spaceId, onUploaded }: { token: string; spaceId: string; onUploaded: () => void }) {
  const [recording, setRecording] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const context = useRef<AudioContext | null>(null);
  const processor = useRef<ScriptProcessorNode | null>(null);
  const source = useRef<MediaStreamAudioSourceNode | null>(null);
  const stream = useRef<MediaStream | null>(null);
  const chunks = useRef<Float32Array[]>([]);

  const stop = () => {
    processor.current?.disconnect(); source.current?.disconnect(); stream.current?.getTracks().forEach((track) => track.stop());
    const audio = context.current; context.current = null; processor.current = null; source.current = null; stream.current = null; setRecording(false);
    if (!audio || chunks.current.length === 0) return;
    const file = encodeWav(chunks.current, audio.sampleRate); chunks.current = []; void audio.close();
    setBusy(true); void uploadVoiceReport(token, spaceId, file).then(onUploaded).catch((cause) => setError(cause instanceof Error ? cause.message : "语音上传失败")).finally(() => setBusy(false));
  };

  const start = async () => {
    try {
      setError(""); chunks.current = [];
      const media = await navigator.mediaDevices.getUserMedia({ audio: true });
      const audio = new AudioContext(); const input = audio.createMediaStreamSource(media); const node = audio.createScriptProcessor(4096, 1, 1); const mute = audio.createGain(); mute.gain.value = 0;
      node.onaudioprocess = (event) => chunks.current.push(new Float32Array(event.inputBuffer.getChannelData(0)));
      input.connect(node); node.connect(mute); mute.connect(audio.destination); stream.current = media; context.current = audio; source.current = input; processor.current = node; setRecording(true);
    } catch (cause) { setError(cause instanceof Error ? cause.message : "无法获得麦克风权限"); }
  };

  return <div className="mt-3 border-t border-slate-200 pt-3" aria-label="语音对讲录音"><div className="flex items-center justify-between gap-2"><div className="flex items-center gap-1 text-xs font-medium text-slate-700"><Mic size={14} />语音对讲录音</div><span className="text-xs text-red-700">{recording ? "正在录音" : "未录音"}</span></div><p className="mt-1 text-xs text-slate-500">录音会生成 WAV 片段并进入待审核队列。</p><Button size="sm" variant={recording ? "danger" : "secondary"} isDisabled={busy} onPress={recording ? stop : () => void start()}>{recording ? <><Square size={14} />结束录音</> : <><Mic size={14} />开始录音</>}</Button>{error && <p className="mt-1 text-xs text-red-700" role="alert">{error}</p>}</div>;
}
