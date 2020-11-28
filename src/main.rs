use anyhow::{Context, Result};
use std::io::BufReader;
use std::path::PathBuf;
use std::{fs::File, io::Read};
use structopt::StructOpt;

/// TODO fill this out
#[derive(StructOpt, Debug)]
#[structopt(name = "padsynth")]
pub struct Opt {
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

fn main() -> Result<()> {
    let opt: Opt = Opt::from_args();

    let _wav_data = {
        let wav_path = &opt.wav;
        let wav_file = File::open(wav_path).with_context(|| {
            format!(
                "cannot open WAV file '{}' specified in command line",
                wav_path.display(),
            )
        })?;
        let mut buf_reader = BufReader::new(wav_file);

        wav::read(&mut buf_reader)
            .with_context(|| format!("reading WAV file '{}'", wav_path.display(),))?
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
        cfg_file
            .read_to_end(&mut cfg_bytes)
            .with_context(|| format!("reading config file '{}'", cfg_path.display()))?;
        drop(cfg_file);

        ron::de::from_bytes::<cfg::Config>(&cfg_bytes)
            .with_context(|| format!("parsing config file '{}'", cfg_path.display()))?
    };

    dbg!(cfg);

    Ok(())
}
