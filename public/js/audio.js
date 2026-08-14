// Synthesized move / capture / win cues. Browsers block audio until a
// gesture, so the AudioContext is created and resumed lazily.

export class AudioManager {
  constructor() {
    this.muted = false;
    this._ctx = null;
  }

  toggleMute() {
    this.muted = !this.muted;
    return this.muted;
  }

  async _ensure() {
    if (!this._ctx) {
      this._ctx = new (window.AudioContext || window.webkitAudioContext)();
    }
    if (this._ctx.state === 'suspended') {
      await this._ctx.resume().catch(() => {});
    }
    return this._ctx;
  }

  play(name) {
    if (this.muted) return;
    this._ensure().then((ctx) => {
      if (!ctx || ctx.state !== 'running') return;
      const now = ctx.currentTime;
      if (name === 'capture') this._tone(ctx, now, 220, 0.16, 0.09);
      else if (name === 'win') {
        this._tone(ctx, now, 523.25, 0.12, 0.07);
        this._tone(ctx, now + 0.12, 659.25, 0.14, 0.07);
        this._tone(ctx, now + 0.26, 783.99, 0.28, 0.08);
      } else {
        this._tone(ctx, now, 392, 0.09, 0.05);
      }
    }).catch(() => {});
  }

  _tone(ctx, when, freq, dur, gain) {
    const osc = ctx.createOscillator();
    const amp = ctx.createGain();
    osc.type = 'triangle';
    osc.frequency.setValueAtTime(freq, when);
    amp.gain.setValueAtTime(0.0001, when);
    amp.gain.exponentialRampToValueAtTime(gain, when + 0.012);
    amp.gain.exponentialRampToValueAtTime(0.0001, when + dur);
    osc.connect(amp);
    amp.connect(ctx.destination);
    osc.start(when);
    osc.stop(when + dur + 0.02);
  }
}
