//! Note: "foo__bar" variable names indicate "foo/bar" unit ratios.
//! Unfortunately Rust does not allow slashes, or Unicode characters, in variable names,
//! and "foo_per_bar" is sometimes too verbose and long.

use anyhow::{bail, Result};
use cfg::{ChordPitch, Duration, Volume};

use crate::cfg;
use crate::cfg::Config;
use crate::{cfg::Pitch, common::Amplitude};

use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;

type RealVec = Vec<f32>;

type FftAmplitude = Complex<f32>;
type FftVec = Vec<FftAmplitude>;
type FftSlice = [FftAmplitude];

/// Computes the total power of several FFT bins,
/// and takes the square root to find the equivalent amplitude.
fn root_sum_power(fft_bins: &FftSlice) -> Amplitude {
    fft_bins
        .iter()
        .map(|c| c.norm_sqr())
        .sum::<Amplitude>()
        .sqrt()
}

/// Like root_sum_power but for real numbers.
fn root_sum_square(arr: &[Amplitude]) -> Amplitude {
    arr.iter().map(|c| c * c).sum::<Amplitude>().sqrt()
}

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

fn chord_pitch_to_freq(pitch: ChordPitch, _fund_freq: Option<f32>) -> Result<f32> {
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

struct Spectrum<T> {
    spectrum: T,

    /// "period" is the number of samples fed into the FFT.
    /// period/s is an unusual choice compared with sample/s,
    /// but is the simplest way to convert from cycle/s (Hz)
    /// to cycle/period (FFT frequency bins).
    period_per_s: f32,
}

fn load_input(
    in_cfg: &cfg::Input,
    mut data: &[Amplitude],
    wav_smp_per_s: u32,
) -> Result<Spectrum<FftVec>> {
    // Trim the wav data to the looped portion.
    let loop_begin = in_cfg.loop_begin;
    let loop_end = in_cfg.loop_end.unwrap_or(data.len());
    if !(loop_end > loop_begin) {
        bail!(
            "loop end = {} must be greater than loop begin {}",
            loop_end,
            loop_begin
        );
    }

    data = &data[loop_begin..loop_end];

    // Take the FFT of the looped portion.
    let mut planner = realfft::RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(data.len());

    let mut spectrum = {
        let mut data_copy = Vec::from(data);
        let mut spectrum = vec![FftAmplitude::zero(); data.len() / 2 + 1];
        fft.process(&mut data_copy, &mut spectrum).unwrap();
        spectrum
    };

    // Normalize spectrum to have constant total power if input is resampled to a different length.
    for ampl in &mut spectrum {
        *ampl /= data.len() as f32;
    }

    let mut smp_per_s = in_cfg.transpose.sample_rate.unwrap_or(wav_smp_per_s) as f32;
    smp_per_s *= cents_to_freq_mul(in_cfg.transpose.detune_cents);
    let smp_per_period = data.len() as f32;

    Ok(Spectrum {
        spectrum,
        period_per_s: smp_per_s / smp_per_period,
    })
}

struct SpectrumAndNote<T> {
    spectrum: T,
    period_per_s: f32,
    cyc_per_s: f32,
}
// &Spectrum<&mut FftSlice> is just sad...
// It's possible to add a "clone-borrow" method (https://gist.github.com/nyanpasu64/285ed17bb8787cf6821e900085c5c38b),
// but this type is 16 bytes, so stack references may be faster than cloning (IDK).

type Random = rand_pcg::Pcg32;

/// Adds power to FFT bins around a specific harmonic (of a note),
/// using a windowed Gaussian curve to distribute power among bins.
///
/// The width of the Gaussian curve is proportional to `stdev_rel * freq`.
/// For a fixed value of stdev_rel, higher harmonics have a wider peak.
/// This models real-life detuned ensembles where for a given instrument detune,
/// each harmonic's frequency deviation is proportional to the harmonic's pitch.
///
/// Only used by SynthMode::Harmonic and PreserveFormants.
/// SynthMode::PreserveSpectrum samples an interpolator over the entire input spectrum,
/// instead of summing the power of input bins into evenly-spaced harmonics
/// and parametrically (stdev_rel) recreating each harmonic's frequency spread.
///
/// Returns Err if the harmonic (and all higher harmonics)
/// lies above the output spectrum's Nyquist frequency and does not contribute to the sound.
fn add_harmonic(
    spectrum: &mut Spectrum<&mut FftSlice>,
    cyc_per_s: f32,
    stdev_rel: f32,
    volume: f32,
    random_amplitudes: bool,
    rng: &mut Random,
) -> Result<(), ()> {
    // Loosely based on https://zynaddsubfx.sourceforge.io/doc/PADsynth/PADsynth.htm.

    // Compute parameters for a Gaussian harmonic.
    // Because the value will be negligible outside a [minimum..maximum) window,
    // produce a Vec<complex> where 0..n maps to minimum..maximum.
    let compute_envelope = || {
        // FFT bins are "cyc/period".
        let center_cyc__period = cyc_per_s / spectrum.period_per_s;
        let stdev_cyc__period = stdev_rel * center_cyc__period;

        /// How many standard deviations away from the center frequency to generate sound.
        const MAX_STDEV: f32 = 3.0;

        let deviation = (stdev_cyc__period * MAX_STDEV).min(center_cyc__period);
        if deviation <= 0.0 {
            // This is invalid and pedantically should be a hard error (nested Result).
            return Err(());
        }

        let min_bin = (center_cyc__period - stdev_cyc__period * MAX_STDEV).ceil() as usize;
        let mut max_bin = (center_cyc__period + stdev_cyc__period * MAX_STDEV).ceil() as usize;

        if min_bin >= spectrum.spectrum.len() {
            return Err(());
        }
        max_bin = max_bin.min(spectrum.spectrum.len());

        let mut envelope = vec![FftAmplitude::zero(); max_bin - min_bin];

        fn gauss(x: f32, loc: f32, scale: f32) -> f32 {
            let xrel = (x - loc) / scale;
            return (-xrel * xrel / 2.0).exp();
        }
        for bin in min_bin..max_bin {
            envelope[bin - min_bin] =
                gauss(bin as f32, center_cyc__period, stdev_cyc__period).into();
        }

        {}

        Ok((envelope, min_bin, max_bin))
    };
    let (mut envelope, min_bin, max_bin) = compute_envelope()?;

    let random_unit_complex = |rng: &mut _| {
        use rand_distr::{Distribution, UnitCircle};
        let [a, b] = UnitCircle.sample(/*mut*/ rng);
        FftAmplitude::new(a, b)
    };

    // Add random phases to the envelope of the harmonic, to produce a spectrum.
    if random_amplitudes {
        unimplemented!("random_amplitudes=true is not implemented");
    } else {
        for x in &mut envelope {
            *x *= random_unit_complex(/*mut*/ rng);
        }
    }
    let mut harmonic_spectrum = envelope;

    // Normalize the harmonic to unit power,
    // and multiply by the amplitude of the band in the input signal.
    let normalize_cplx = |data: &mut [_]| {
        let sum = root_sum_power(data);
        let scaling_factor = volume / sum;
        for a in data {
            *a *= scaling_factor;
        }
    };
    normalize_cplx(&mut harmonic_spectrum);

    // TODO add API to just return (spectrum, min bin, max bin)? Is this useful or not?

    // Add the harmonic to the global spectrum.
    assert_eq!(harmonic_spectrum.len(), max_bin - min_bin);
    for (global_ampl, &harmonic_ampl) in spectrum.spectrum[min_bin..max_bin]
        .iter_mut()
        .zip(&harmonic_spectrum)
    {
        *global_ampl += harmonic_ampl;
    }

    Ok(())
}

/// For a given output note, generate all harmonics
/// and add them one-by-one to the output spectrum.
///
/// Entry 0 of `harmonic_amplitudes` corresponds to harmonic 1. DC is not included.
/// Only used when mode is SynthMode::Harmonic.
fn add_note_direct(
    harmonic_amplitudes: &[Amplitude],
    output_note: &mut SpectrumAndNote<&mut FftSlice>,
    stdev_rel: f32,
    volume: f32,
    random_amplitudes: bool,
    rng: &mut Random,
) {
    for (harmonic, &amplitude) in (1..).zip(harmonic_amplitudes) {
        if add_harmonic(
            &mut Spectrum {
                spectrum: /*mut*/ output_note.spectrum,
                period_per_s: output_note.period_per_s,
            },
            output_note.cyc_per_s * harmonic as f32,
            stdev_rel,
            volume * amplitude,
            random_amplitudes,
            /*mut*/ rng,
        )
        .is_err()
        {
            break;
        }
    }
}

/// Extracts the power of each harmonic,
/// and takes the square root to find the equivalent amplitude.
///
/// Entry 0 of the return value corresponds to harmonic 1. DC is not included.
fn note_to_harmonics(input_note: &SpectrumAndNote<&FftSlice>) -> Vec<Amplitude> {
    // FFT bins are "cyc/period".
    let cyc_per_period = input_note.cyc_per_s / input_note.period_per_s;

    // ceil() is used to convert floating-point bin indices into half-open range endpoints.
    let harmonic_to_fft_bin = |h: f32| (h * cyc_per_period).ceil() as usize;

    let mut output: Vec<Amplitude> = vec![];

    let spectrum: &[FftAmplitude] = input_note.spectrum;
    for harmonic in 1.. {
        let bottom_bin = harmonic_to_fft_bin(harmonic as f32 - 0.5);
        if bottom_bin >= spectrum.len() {
            break;
        }

        let mut top_bin = harmonic_to_fft_bin((harmonic + 1) as f32 - 0.5);
        top_bin = top_bin.min(spectrum.len());

        let harmonic_range = &spectrum[bottom_bin..top_bin];
        let total_amplitude = root_sum_power(harmonic_range);
        output.push(total_amplitude);
    }

    output
}

fn synthesize(out_cfg: &cfg::Output, input_note: SpectrumAndNote<&FftSlice>) -> Result<RealVec> {
    use realfft::num_complex::ComplexFloat;

    // Setup state based on out_cfg.
    let out_nsamp = duration_to_samples(out_cfg.duration, out_cfg.sample_rate);
    let fund_freq: Option<f32> = match out_cfg.mode {
        // TODO if PreserveFormants(fund_pitch), return Some(pitch_to_freq(fund_pitch)).
        _ => None,
    };
    let master_volume = volume_to_ampl(out_cfg.master_volume);
    let random_amplitudes = out_cfg.random_amplitudes;

    // Setup state for time/frequency conversion..
    let out_smp_per_s = out_cfg.sample_rate as f32;
    let out_smp_per_period = out_nsamp as f32;

    // Seed RNG.
    use rand::SeedableRng;
    let mut rng = Random::seed_from_u64(out_cfg.seed);

    // Initialize spectrum to all zeros.
    let mut out_spectrum = vec![FftAmplitude::zero(); out_nsamp / 2 + 1];

    // Fill spectrum with each note.
    for note in &out_cfg.chord {
        let cyc_per_s = chord_pitch_to_freq(note.pitch, fund_freq)?;
        let volume = master_volume * volume_to_ampl(note.volume);

        use cfg::SynthMode;
        match out_cfg.mode {
            SynthMode::Harmonic { stdev } => {
                let harmonic_amplitudes = note_to_harmonics(&input_note);
                add_note_direct(
                    &harmonic_amplitudes,
                    &mut SpectrumAndNote {
                        spectrum: &mut out_spectrum,
                        period_per_s: out_smp_per_s / out_smp_per_period,
                        cyc_per_s,
                    },
                    stdev,
                    volume,
                    random_amplitudes,
                    &mut rng,
                );
            }
        }
    }

    if out_nsamp % 2 == 0 {
        let nyquist: &mut FftAmplitude = out_spectrum.last_mut().unwrap();
        *nyquist = FftAmplitude::new(nyquist.abs().copysign(nyquist.re()), 0.);
    }

    let mut planner = realfft::RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_inverse(out_nsamp);
    let mut out_data = vec![0f32; out_nsamp];
    fft.process(&mut out_spectrum, &mut out_data).unwrap();
    Ok(out_data)
}

/// `cfg` represents the padsynth setup, read from a config file.
///
/// `data` represents the entire wav file, downmixed to mono,
/// but not yet trimmed to the looped section only.
///
pub fn process(cfg: &Config, data: &[Amplitude], wav_smp_per_s: u32) -> Result<RealVec> {
    let in_cfg = &cfg.input;

    let spectrum = load_input(&cfg.input, data, wav_smp_per_s)?;
    let freq = pitch_to_freq(in_cfg.pitch);

    let out = synthesize(
        &cfg.output,
        SpectrumAndNote {
            spectrum: &spectrum.spectrum,
            period_per_s: spectrum.period_per_s,
            cyc_per_s: freq,
        },
    )?;
    Ok(out)
}
