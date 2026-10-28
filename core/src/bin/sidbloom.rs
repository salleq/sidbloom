//! Command line front end: render, export and demo files.
use sidcore::audio::{render_wav, SidModel};
use sidcore::export::{export_prg, export_sid};
use sidcore::project;
use std::{env, fs, path::Path, process::ExitCode};

const USAGE: &str = "usage:
  sidbloom demo <out.sidb>                       write the demo project
  sidbloom render <in.sidb> <out.wav> [seconds]  render to WAV (reSID, 8580)
  sidbloom export-sid <in.sidb> <out.sid> [--8580]
  sidbloom export-prg <in.sidb> <out.prg>
  sidbloom import-swm <in.swm|.prg|.p00> <out.sidb>   import a SID-Wizard 1.x module";

fn run(args: &[String]) -> Result<(), String> {
    let a = |i: usize| args.get(i).map(String::as_str).ok_or_else(|| USAGE.to_string());
    match a(0)? {
        "demo" => project::save(&project::demo_song(), Path::new(a(1)?)).map_err(|e| e.to_string()),
        "render" => {
            let song = project::load(Path::new(a(1)?)).map_err(|e| e.to_string())?;
            let secs: f64 = args.get(3).map_or(Ok(10.0), |s| s.parse()).map_err(|_| "bad seconds")?;
            fs::write(a(2)?, render_wav(&song, secs, SidModel::Mos8580)).map_err(|e| e.to_string())
        }
        "export-sid" => {
            let song = project::load(Path::new(a(1)?)).map_err(|e| e.to_string())?;
            let is8580 = args.iter().any(|x| x == "--8580");
            fs::write(a(2)?, export_sid(&song, is8580).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
        }
        "export-prg" => {
            let song = project::load(Path::new(a(1)?)).map_err(|e| e.to_string())?;
            fs::write(a(2)?, export_prg(&song).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
        }
        "import-swm" => {
            let bytes = fs::read(a(1)?).map_err(|e| e.to_string())?;
            let imp = sidcore::import::swm::import_swm(&bytes).map_err(|e| e.to_string())?;
            for w in &imp.warnings {
                eprintln!("note: {w}");
            }
            project::save(&imp.song, Path::new(a(2)?)).map_err(|e| e.to_string())
        }
        _ => Err(USAGE.into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
