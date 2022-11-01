Installation:

```sh
cargo install --git https://github.com/nyanpasu64/padsynth.git
# If you cloned the repo:
cargo install --path .
```

Usage: `padsynth <WAV> <CONFIG> <OUT_WAV>`

- `<WAV>        Audio file to resynthesize`
- `<CONFIG>     Configuration file for resynthesis`
- `<OUT_WAV>    Output file to write to`

`CONFIG` files are in [RON format](https://docs.rs/ron/latest/ron/), and are conventionally named `name.pad-cfg`. For the schema, see `src/cfg.rs`. Example:

```rust
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
