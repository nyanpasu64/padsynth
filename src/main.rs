use anyhow::{Context, Result};
use std::{fs::File, io::BufReader, path::PathBuf};
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

type WavReader = hound::WavReader<BufReader<File>>;

struct WavToFloat {
    wav: WavReader,
}

impl WavToFloat {
    fn new(wav: WavReader) -> WavToFloat {
        WavToFloat{wav}
    }

    fn read(&mut self) -> Result<Vec<f32>> {
        let spec = self.wav.spec();

        match (spec.sample_format, spec.bits_per_sample) {
            (hound::SampleFormat::Float, 32) => {
                Ok(self.wav.samples::<f32>().collect())
            },
            _ => anyhow::bail!(
                "Unsupported sample format {:?}{}",
                spec.sample_format,
                spec.bits_per_sample,
            ),
        }
    }
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let wav = WavReader::open(&opt.wav).with_context(|| {
        format!(
            "cannot open WAV file '{}' specified in command line",
            opt.wav.display(),
        )
    })?;


    let _wav_data = ;

    println!("{:?}", opt);

    Ok(())
}
