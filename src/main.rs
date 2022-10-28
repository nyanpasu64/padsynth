#![allow(non_snake_case)]

use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use structopt::StructOpt;
use wav::BitDepth;

/// TODO fill this out
#[derive(StructOpt, Debug)]
#[structopt(name = "padsynth")]
struct Opt {
    /// Audio file to resynthesize
    #[structopt(name = "WAV", parse(from_os_str))]
    wav: PathBuf,

    /// Configuration file for resynthesis
    #[structopt(name = "CONFIG", parse(from_os_str))]
    cfg: PathBuf,

    /// Output file to write to
    #[structopt(name = "OUT_WAV", parse(from_os_str))]
    out_wav: PathBuf,
}

mod cfg;

mod common {
    pub type Amplitude = f32;
}

mod dsp;

use common::Amplitude;

fn wav_to_float(wav: BitDepth) -> Vec<Amplitude> {
    fn u8_to_i8(x: u8) -> i8 {
        (x ^ 0x80) as i8
    }

    match wav {
        BitDepth::Eight(v) => v
            .into_iter()
            .map(|a: u8| u8_to_i8(a) as Amplitude / (1 << 7) as Amplitude)
            .collect(),
        BitDepth::Sixteen(v) => v
            .into_iter()
            .map(|a: i16| a as Amplitude / (1 << 15) as Amplitude)
            .collect(),
        BitDepth::TwentyFour(v) => v
            .into_iter()
            .map(|a: i32| a as Amplitude / (1 << 23) as Amplitude)
            .collect(),
        BitDepth::ThirtyTwoFloat(v) => v,
        BitDepth::Empty => {
            unreachable!("wav::read() never returns Ok(BitDepth::Empty) but Err instead")
        }
    }
}

fn float_to_i16(data: &[Amplitude]) -> Result<Vec<i16>> {
    let mut out = vec![0i16; data.len()];
    for (f, i) in data.iter().zip(&mut out) {
        let f = (f * (1 << 15) as f32).round();
        *i = f as i16;
        if *i as f32 != f {
            bail!("Error, clipping detected when writing WAV file");
        }
    }
    Ok(out)
}

fn downmix_wav(header: &wav::Header, data: Vec<Amplitude>) -> Vec<Amplitude> {
    let nchan = header.channel_count as usize;
    assert!(nchan >= 1);
    if nchan == 1 {
        data
    } else {
        assert!(data.len() % nchan == 0);
        data.chunks_exact(nchan)
            .map(|frame| frame.iter().sum::<f32>() / nchan as f32)
            .collect()
    }
}

fn main() -> Result<()> {
    let opt: Opt = Opt::from_args();

    let (header, data): (wav::Header, Vec<Amplitude>) = {
        let wav_path = &opt.wav;
        let wav_file = File::open(wav_path).with_context(|| {
            format!(
                "cannot open WAV file '{}' specified in command line",
                wav_path.display(),
            )
        })?;
        let mut buf_reader = BufReader::new(wav_file);

        let (header, data) = wav::read(&mut buf_reader)
            .with_context(|| format!("reading WAV file '{}'", wav_path.display(),))?;
        (header, wav_to_float(data))
    };

    let data = downmix_wav(&header, data);

    let cfg = {
        let cfg_path = &opt.cfg;
        let mut cfg_file = File::open(cfg_path).with_context(|| {
            format!(
                "cannot open config file '{}' specified in command line",
                opt.cfg.display(),
            )
        })?;

        let mut cfg_bytes = Vec::new();
        use std::io::Read;
        cfg_file
            .read_to_end(&mut cfg_bytes)
            .with_context(|| format!("reading config file '{}'", cfg_path.display()))?;
        drop(cfg_file);

        ron::de::from_bytes::<cfg::Config>(&cfg_bytes)
            .with_context(|| format!("parsing config file '{}'", cfg_path.display()))?
    };
    cfg.validate()?;

    let out_data = dsp::process(&cfg, &data, header.sampling_rate)?;

    let out_wav_data = float_to_i16(&out_data)?;
    drop(out_data);

    {
        let out_wav_path = &opt.out_wav;
        let out_file = File::create(out_wav_path)?;
        let mut buf_writer = BufWriter::new(out_file);
        wav::write(
            wav::Header::new(
                1, // audio format = PCM
                1, // channel count = 1
                cfg.output.sample_rate,
                16, // bits/sample
            ),
            &BitDepth::Sixteen(out_wav_data),
            &mut buf_writer,
        )
        .with_context(|| format!("writing WAV file '{}'", out_wav_path.display()))?;

        use std::io::Write;
        buf_writer
            .flush()
            .with_context(|| format!("flushing output WAV file '{}'", out_wav_path.display()))?;
    }

    Ok(())
}
