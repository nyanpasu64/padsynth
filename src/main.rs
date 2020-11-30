#![allow(non_snake_case)]

use anyhow::{Context, Result};
use std::fs::File;
use std::io::BufReader;
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
        BitDepth::Empty => {
            unreachable!("wav::read() never returns Ok(BitDepth::Empty) but Err instead")
        }
    }
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

    let data = downmix_wav(&header, data);

    let out_data = dsp::process(&cfg, &data, header.sampling_rate);

    // TODO write to wav

    Ok(())
}
