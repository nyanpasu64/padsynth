use serde::Deserialize;

/// File config struct.
#[derive(Deserialize, Debug)]
pub struct Config {
    pub input: Input,
    pub output: Output,
}

#[derive(Deserialize, Debug)]
pub struct Input {
    /// TODO If omitted, smpl chunk must be present, and will be used for loop begin.
    pub loop_begin: usize,

    /// Not included in loop.
    /// If omitted, defaults to smpl chunk if present, otherwise end of sample.
    #[serde(default)]
    pub loop_end: Option<usize>,

    pub transpose: Transpose,

    /// Used to split the input signal up into bins.
    pub pitch: Pitch,
}

#[derive(Deserialize, Debug, Default)]
pub struct Transpose {
    #[serde(default)]
    pub sample_rate: Option<u32>,

    #[serde(default)]
    pub detune_cents: f32,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum Pitch {
    Hz(f32),
    Midi(i32),
}

#[derive(Deserialize, Debug)]
pub struct Output {
    pub sample_rate: u32,
    pub duration: Duration,
    pub mode: SynthMode,

    #[serde(default)]
    pub master_volume: Volume,
    pub chord: Vec<ChordNote>,

    #[serde(default)]
    pub seed: u64,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum SynthMode {
    // TODO PreserveSpectrum,
    Harmonic { stdev: f32 },
    // TODO PreserveFormants { stdev: f32, fund_pitch: Pitch },
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum Duration {
    Smp(usize),
    TimeMs(f32),
}

#[derive(Deserialize, Debug)]
pub struct ChordNote {
    pub pitch: ChordPitch,
    pub volume: Volume,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum ChordPitch {
    // TODO Harmonic(f32), (only valid if harmonic_stdev is Some)
    Hz(f32),
    Midi(i32),
}

#[derive(Deserialize, Debug, Clone, Copy)]
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
