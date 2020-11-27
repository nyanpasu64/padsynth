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

fn main() {
    let opt = Opt::from_args();

    println!("{:?}", opt);
}
