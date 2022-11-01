```
USAGE:
    padsynth <WAV> <CONFIG> <OUT_WAV>

ARGS:
    <WAV>        Audio file to resynthesize
    <CONFIG>     Configuration file for resynthesis
    <OUT_WAV>    Output file to write to
```

CONFIG = `name.pad-cfg` (see `src/cfg.rs`):

```ron
(
    input: (
        loop_begin: 48,
        transpose: (
            detune_cents: -10,
        ),
        pitch: Midi(60),
    ),
    output: (
        sample_rate: 10000,
        duration: Smp(8192),  // 8 * 1024
        mode: Harmonic(stdev: 0.012),
        master_volume: Power(0.5),
        random_amplitudes: false,
        chord: [
            (
                pitch: Midi(60),
                volume: Ampl(1),
            ),
            (
                pitch: Midi(67),
                volume: Ampl(1),
            ),
        ],
        seed: 0,
    ),
)
```
