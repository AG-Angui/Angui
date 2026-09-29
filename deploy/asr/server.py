import asyncio
import io
import os
from fastapi import FastAPI, HTTPException, Request
from fastapi.concurrency import run_in_threadpool
from faster_whisper import WhisperModel

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
model = WhisperModel(
    os.environ.get("ASR_MODEL", "small"),
    device=os.environ.get("ASR_DEVICE", "cpu"),
    compute_type=os.environ.get("ASR_COMPUTE_TYPE", "int8"),
    download_root="/models",
)
lock = asyncio.Semaphore(1)


@app.get("/health")
def health():
    return {"status": "ok", "model": os.environ.get("ASR_MODEL", "small")}


def decode(data: bytes):
    segments, _ = model.transcribe(io.BytesIO(data), language="zh", vad_filter=True)
    return " ".join(segment.text.strip() for segment in segments).strip()


@app.post("/transcribe")
async def transcribe(request: Request):
    key = os.environ.get("ASR_API_KEY")
    if not key or request.headers.get("authorization") != f"Bearer {key}":
        raise HTTPException(status_code=401, detail="unauthorized")
    if request.headers.get("content-type") != "application/octet-stream":
        raise HTTPException(status_code=415, detail="audio bytes required")
    data = await request.body()
    if not data or len(data) > 10 * 1024 * 1024:
        raise HTTPException(status_code=413, detail="audio size invalid")
    async with lock:
        try:
            text = await run_in_threadpool(decode, data)
        except Exception:
            raise HTTPException(status_code=422, detail="audio decoding failed") from None
    return {"text": text, "model": os.environ.get("ASR_MODEL", "small")}
