# Optional ElevenLabs `signal` overlays

These are the user-selected, original ElevenLabs effects converted to the runtime's required
48 kHz, mono, signed-16-bit WAV format:

- `signal_elevenlabs_a.wav` — selected take 1, approximately 2 seconds.
- `signal_elevenlabs_c.wav` — selected take 3, approximately 1 second.

They are intentionally optional. `sounds ensure-bank` regenerates the per-duck synthesized bank,
so install these only *after* it has run:

```sh
install -D -m 0644 sounds/external/signal/signal_elevenlabs_a.wav /var/lib/robot/sounds/signal/signal_elevenlabs_a.wav
install -D -m 0644 sounds/external/signal/signal_elevenlabs_c.wav /var/lib/robot/sounds/signal/signal_elevenlabs_c.wav
```

`robot.sound` with tag `signal` then selects from the synthesized `signal` variants and these two
overlay files. They do not replace the offline synthesized fallback or change any volume controls.
