use anyhow::{bail, Result};
use cfg::{ChordPitch, Duration, Volume};

use crate::cfg;
use crate::cfg::Config;
use crate::{cfg::Pitch, common::Amplitude};

use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;

type RealVec = Vec<f32>;

type FftSample = Complex<f32>;
type FftVec = Vec<FftSample>;
type FftSlice = [FftSample];

fn cents_to_freq_mul(cents: f32) -> f32 {
    2.0f32.powf(cents / 1200.0)
}

fn midi_to_freq(midi: i32) -> f32 {
    440.0 * 2.0f32.powf((midi - 69) as f32 / 12.0)
}

fn pitch_to_freq(pitch: Pitch) -> f32 {
    match pitch {
        Pitch::Hz(f) => f,
        Pitch::Midi(midi) => midi_to_freq(midi),
    }
}

fn chord_pitch_to_freq(pitch: ChordPitch, fund_freq: Option<f32>) -> Result<f32> {
    match pitch {
        ChordPitch::Hz(f) => Ok(f),
        ChordPitch::Midi(midi) => Ok(midi_to_freq(midi)),
        // TODO if ChordPitch::Harmonic, and fund_freq not present, return error.
    }
}

type SampleIndex<T = usize> = T;

fn duration_to_samples(duration: Duration, smp_per_s: u32) -> SampleIndex {
    const S_OVER_MS: f32 = 1. / 1000.;
    match duration {
        Duration::Smp(smp) => smp,
        Duration::TimeMs(time_ms) => {
            (time_ms * S_OVER_MS * smp_per_s as f32).round() as SampleIndex
        }
    }
}

fn volume_to_ampl(volume: Volume) -> Amplitude {
    match volume {
        Volume::Ampl(ampl) => ampl,
        Volume::Power(power) => power.sqrt(),
        Volume::Db(db) => 10f32.powf(db / 10.).sqrt(),
    }
}

/// `cfg` represents the padsynth setup, read from a config file.
///
/// `data` represents the entire wav file, downmixed to mono,
/// but not yet trimmed to the looped section only.
///
pub fn process(cfg: &Config, data: &[Amplitude], orig_sample_rate: u32) -> Result<RealVec> {
    let input = &cfg.input;

    let mut sample_rate = input.transpose.sample_rate.unwrap_or(orig_sample_rate) as f32;
    sample_rate *= cents_to_freq_mul(input.transpose.detune_cents);
    let pitch = pitch_to_freq(input.pitch);

    fn load_input(input: &cfg::Input, mut data: &[Amplitude]) -> Result<FftVec> {
        // Trim the wav data to the looped portion.
        let loop_begin = input.loop_begin;
        let loop_end = input.loop_end.unwrap_or(data.len());
        if !(loop_end > loop_begin) {
            bail!(
                "loop end = {} must be greater than loop begin {}",
                loop_end,
                loop_begin
            );
        }

        data = &data[loop_begin..loop_end];

        // Take the FFT of the looped portion.
        // We cannot use realfft, because it assumes the input has even length
        // (which may not be true for arbitrary looped samples).
        // It is true for samples ripped from SNES games (multiple of 16).
        let mut fft = realfft::RealToComplex::<f32>::new(loop_end - loop_begin).unwrap();
        let mut data_copy = Vec::from(data);
        let mut spectrum = vec![FftSample::zero(); data.len() / 2 + 1];
        fft.process(&mut data_copy, &mut spectrum).unwrap();
        Ok(spectrum)
    }

    let spectrum = load_input(&cfg.input, data)?;

    struct NoteSpectrum<T> {
        spectrum: T,
        sample_rate: f32,
        pitch: f32,
    }

    fn synthesize(out_cfg: &cfg::Output, input: NoteSpectrum<&FftSlice>) -> Result<RealVec> {
        let out_nsamp = duration_to_samples(out_cfg.duration, out_cfg.sample_rate);
        let fund_freq: Option<f32> = match out_cfg.mode {
            // TODO if PreserveFormants(fund_pitch), return Some(pitch_to_freq(fund_pitch)).
            _ => None,
        };

        use cfg::SynthMode;
        type Random = (); // TODO pick random library

        /// For a given output note, generate all harmonics
        /// and add them one-by-one to the output spectrum.
        fn add_note(
            input: &NoteSpectrum<&FftSlice>,
            output: &NoteSpectrum<&mut FftSlice>,
            volume: f32,
            synth_mode: SynthMode,
            rng: &mut Random,
        ) {
            // Loosely based on https://zynaddsubfx.sourceforge.io/doc/PADsynth/PADsynth.htm.

            // TODO: For SynthMode::Harmonic:
            // Compute power of each input harmonic.
            // For each harmonic:
            // - Compute parameters for a Gaussian harmonic
            // - The value will be negligible outside a [minimum..maximum) window
            // - Produce a Vec<complex> where 0..n maps to minimum..maximum
            // - For each entry, write a spectral component with fixed magnitude and random phase
            //   (or a random Gaussian 2D vector, idk)
            // - Divide all entries in the Vec by entries.norm.^2.sum.sqrt
            // - Multiply by power of band in input.
            //
            // TODO "power of complex slice" function
        }

        let mut rng = Random::default();

        // Initialize spectrum to all zeros.
        let mut out_spectrum = vec![FftSample::zero(); out_nsamp / 2 + 1];

        // Fill spectrum with each note.
        for note in &out_cfg.chord {
            let pitch = chord_pitch_to_freq(note.pitch, fund_freq)?;
            let volume = volume_to_ampl(note.volume);

            add_note(
                &input,
                &NoteSpectrum {
                    spectrum: &mut out_spectrum,
                    sample_rate: out_cfg.sample_rate as f32,
                    pitch,
                },
                volume,
                out_cfg.mode,
                &mut rng,
            );
        }

        let mut fft = realfft::ComplexToReal::<f32>::new(out_nsamp).unwrap();
        let mut out_data = vec![0f32; out_nsamp];
        fft.process(&mut out_spectrum, &mut out_data).unwrap();
        Ok(out_data)
    }

    let out = synthesize(
        &cfg.output,
        NoteSpectrum {
            spectrum: &spectrum,
            sample_rate,
            pitch,
        },
    )?;
    Ok(out)
}
