use serde::Deserialize;

/// File config struct.
#[derive(Deserialize, Debug)]
pub struct Config {
    input: Input,
    output: Output,
}

#[derive(Deserialize, Debug)]
pub struct Input {
    /// TODO If omitted, smpl chunk must be present, and will be used for loop begin.
    loop_begin: u32,

    /// Not included in loop.
    /// If omitted, defaults to smpl chunk if present, otherwise end of sample.
    #[serde(default)]
    loop_end: Option<u32>,

    transpose: Transpose,

    /// Used to split the input signal up into bins.
    pitch: Pitch,
}

#[derive(Deserialize, Debug, Default)]
pub struct Transpose {
    #[serde(default)]
    sample_rate: Option<u32>,

    #[serde(default)]
    detune_cents: f32,
}

#[derive(Deserialize, Debug)]
pub enum Pitch {
    Hz(u32),
    Midi(i32),
}

#[derive(Deserialize, Debug)]
pub struct Output {
    sample_rate: u32,
    duration: Duration,
    // TODO mode: PreserveSpectrum | Harmonic{stdev} | PreserveFormants{stdev, fund_pitch}
    // and remove harmonic_stdev
    harmonic_stdev: f32,

    #[serde(default)]
    master_volume: Volume,
    chord: Vec<ChordNote>,
}

#[derive(Deserialize, Debug)]
pub enum Duration {
    Smp(u32),
    TimeMs(f32),
}

#[derive(Deserialize, Debug)]
pub struct ChordNote {
    pitch: ChordPitch,
    volume: Volume,
}

#[derive(Deserialize, Debug)]
pub enum ChordPitch {
    // TODO Harmonic(f32), (only valid if harmonic_stdev is Some)
    Hz(u32),
    Midi(i32),
}

#[derive(Deserialize, Debug)]
pub enum Volume {
    Ampl(f32),
    Power(f32),
    Db(f32),
}

impl Default for Volume {
    fn default() -> Self {
        Volume::Ampl(1.0)
    }
}
