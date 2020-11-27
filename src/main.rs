use anyhow::{Context, Result};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
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
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let wav_error = || {
        format!(
            "cannot open WAV file '{}' specified in command line",
            opt.wav.display(),
        )
    };

    let file = File::open(&opt.wav).with_context(&wav_error)?;
    let mut buf_reader = BufReader::new(file);

    let _wav_data = wav::read(&mut buf_reader).with_context(&wav_error)?;
    // println!("{:?}", wav_data);

    Ok(())
}
