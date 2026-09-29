class PcmRecorder extends AudioWorkletProcessor {
  constructor() {
    super();
    this.buffer = new Float32Array(4096);
    this.offset = 0;
    this.port.onmessage = (event) => {
      if (event.data?.type !== "flush") return;
      if (this.offset > 0) this.port.postMessage(this.buffer.slice(0, this.offset));
      this.buffer = new Float32Array(4096);
      this.offset = 0;
      this.port.postMessage({ type: "flushed" });
    };
  }
  process(inputs) {
    const channels = inputs[0];
    if (channels && channels[0]) {
      const samples = channels[0];
      for (let i = 0; i < samples.length; i += 1) {
        this.buffer[this.offset++] = samples[i];
        if (this.offset === this.buffer.length) {
          this.port.postMessage(this.buffer, [this.buffer.buffer]);
          this.buffer = new Float32Array(4096);
          this.offset = 0;
        }
      }
    }
    return true;
  }
}
registerProcessor("pcm-recorder", PcmRecorder);
