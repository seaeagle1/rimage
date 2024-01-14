use std::{error::Error, path::PathBuf, str::FromStr, process::Stdio, thread};
use std::io::{BufReader, BufRead, Write};
use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{arg, value_parser, ArgAction, Command};

#[cfg(feature = "parallel")]
use rayon::{iter::IntoParallelIterator, iter::ParallelIterator};

use paths::collect_files;
use rimage::config::{Codec, EncoderConfig, QuantizationConfig, ResizeConfig, ResizeType};

mod optimize;
mod paths;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("rimage")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Vladyslav Vladinov <vladinov.dev@gmail.com>")
        .about("A tool to convert/optimize/resize images in different formats")
        .arg(
            arg!(<FILES> "Input file(s) to process")
                .num_args(1..)
                .value_delimiter(None)
                .value_parser(value_parser!(PathBuf)),
        )
        .next_help_heading("General")
        .args([
            arg!(-q --quality <QUALITY> "Optimization image quality, disabled when use Jpegxl format\n[range: 1 - 100]")
                .value_parser(value_parser!(f32))
                .default_value("75"),
            arg!(-f --codec <CODEC> "Image codec to use\n[possible values: png, oxipng, jpegxl, webp, avif]")
                .value_parser(Codec::from_str)
                .default_value("mozjpeg"),
            arg!(-o --output <DIR> "Write output file(s) to <DIR>, if \"-r\" option is not used")
                .value_parser(value_parser!(PathBuf)),
            arg!(-r --recursive "Saves output file(s) preserving folder structure")
                .action(ArgAction::SetTrue),
            arg!(-s --suffix [SUFFIX] "Appends suffix to output file(s) names"),
            arg!(-b --backup "Appends \".backup\" suffix to input file(s) extension")
                .action(ArgAction::SetTrue),
            #[cfg(feature = "parallel")]
            arg!(-t --threads <NUM> "Number of threads to use\n[range: 1 - 16] [default: number of cores]")
                .value_parser(value_parser!(usize)),
        ])
        .next_help_heading("Quantization")
        .args([
            arg!(--quantization [QUALITY] "Enables quantization with optional quality\n[range: 1 - 100] [default: 75]")
                .value_parser(value_parser!(u8).range(..=100))
                .default_missing_value("75"),
            arg!(--dithering [QUALITY] "Enables dithering with optional quality\n[range: 1 - 100] [default: 75]")
                .value_parser(value_parser!(f32))
                .default_missing_value("75")
        ])
        .next_help_heading("Resizing")
        .args([
            arg!(--width <WIDTH> "Resize image with specified width\n[integer only]")
                .value_parser(value_parser!(usize)),
            arg!(--height <HEIGHT> "Resize image with specified height\n[integer only]")
                .value_parser(value_parser!(usize)),
            arg!(--filter <FILTER> "Filter used for image resizing\n[possible values: point, triangle, catrom, mitchell]")
                .value_parser(ResizeType::from_str)
                .default_value("lanczos3")
        ])
        .get_matches();

    let codec = matches.get_one::<Codec>("codec").unwrap();
    let quality = matches.get_one::<f32>("quality").unwrap();

    #[cfg(feature = "parallel")]
    if let Some(threads) = matches.get_one::<usize>("threads") {
        rayon::ThreadPoolBuilder::new()
            .num_threads(*threads)
            .build_global()
            .unwrap();
    }

    let mut quantization_config = QuantizationConfig::new();

    if let Some(quality) = matches.get_one::<u8>("quantization") {
        quantization_config = quantization_config.with_quality(*quality)?
    }

    if let Some(dithering) = matches.get_one::<f32>("dithering") {
        quantization_config = quantization_config.with_dithering(*dithering / 100.0)?
    }

    let resize_filter = matches.get_one::<ResizeType>("filter").unwrap();

    let mut resize_config = ResizeConfig::new(*resize_filter);

    if let Some(width) = matches.get_one::<usize>("width") {
        resize_config = resize_config.with_width(*width);
    }

    if let Some(height) = matches.get_one::<usize>("height") {
        resize_config = resize_config.with_height(*height);
    }

    let mut conf = EncoderConfig::new(*codec).with_quality(*quality)?;

    if matches.get_one::<u8>("quantization").is_some()
        || matches.get_one::<f32>("dithering").is_some()
    {
        conf = conf.with_quantization(quantization_config);
    }

    if matches.get_one::<usize>("width").is_some() || matches.get_one::<usize>("height").is_some() {
        conf = conf.with_resize(resize_config);
    }

    let files = matches
        .get_many::<PathBuf>("FILES")
        .unwrap_or_default()
        .map(|v| v.into())
        .collect();

    let out_dir = matches.get_one::<PathBuf>("output").map(|p| p.into());
    let suffix = matches.get_one::<String>("suffix").map(|p| p.into());
    let recursive = matches.get_one::<bool>("recursive").unwrap_or(&false);
    let backup = matches.get_one::<bool>("backup").unwrap_or(&false);
    let filelist = collect_files(files);

    optimize::optimize_files(
        paths::get_paths(
            filelist.clone(),
            out_dir.clone(),
            suffix.clone(),
            codec.to_extension(),
            *recursive,
        ),
        conf,
        *backup,
    );

//    #[cfg(feature = "exiftool")]
    {
        #[cfg(feature = "parallel")]
        let path_vector: Vec<_> = paths::get_paths(
            filelist,
            out_dir,
            suffix,
            codec.to_extension(),
            *recursive,
        ).into_par_iter().collect();

        #[cfg(not(feature = "parallel"))]
        let path_vector = paths::get_paths(
            filelist,
            out_dir,
            suffix,
            codec.to_extension(),
            *recursive,
        );

        exiftool_copy_metadata(path_vector, *backup)?;
    }

    Ok(())
}

//#[cfg(feature = "exiftool")]
fn exiftool_copy_metadata(
    iterator: impl IntoIterator<Item = (PathBuf, PathBuf)>,
    backup: bool,
) -> Result<(), Box<dyn Error>> {
    let (stdout_transmitter, rx) = mpsc::channel();

    let exiftool_process_result = std::process::Command::new("exiftool")
        .args(["-stay_open", "true", "-@", "-"])
        .stdin(Stdio::piped() )
        .stdout( Stdio::piped() )
        .stderr( Stdio::piped() )
        .spawn();

    if exiftool_process_result.is_err() {
        println!("This build of rimage requires exiftool (https://exiftool.org) available in the working directory.");
    }
    let mut exiftool_process = exiftool_process_result?;

    // Take ownership of stdout so we can pass to a separate thread.
    let exiftool_stdout = exiftool_process
        .stdout
        .take()
        .expect("Could not take stdout");

    // Take ownership of stdin so we can pass to a separate thread.
    let exiftool_stderr = exiftool_process
        .stderr
        .take()
        .expect("Could not take stderr");

    // Grab stdin so we can pipe commands to ExifTool
    let exiftool_stdin = exiftool_process.stdin.as_mut().unwrap();

    // Create a separate thread to loop over stdout
    let stop_signal = Arc::new(AtomicBool::new(false));
    let _stdout_thread = thread::spawn({ let stop_stdout = stop_signal.clone();
                                           move || {
        let stdout_lines = BufReader::new(exiftool_stdout).lines();

        for line in stdout_lines {
            let line = line.unwrap();

            if stop_stdout.load(Ordering::SeqCst) {
                return;
            }

            // Check to see if our processing has finished, if it has we will send a message to our main thread.
            if line=="{ready}" {
                stdout_transmitter.send(line).unwrap();
            }
            else {
                // Do some processing out the output from our command. In this case we will just print it.
                println!("exiftool: {}", line);
            }
        }
    }});

    // Create a separate thread to loop over stderr
    // Anything which comes through stderr will be send to console.
    let _stderr_thread = thread::spawn({ let stop_stderr = stop_signal.clone();
                                           move || {
        let stderr_lines = BufReader::new(exiftool_stderr).lines();
        for line in stderr_lines {
            let line = line.unwrap();
            println!("exiftool: {}", line);

            if stop_stderr.load(Ordering::SeqCst) {
                return
            }
        }
        }
    });

    // Loop over target files
    iterator.into_iter()
        .for_each( |(mut input, output): (PathBuf, PathBuf)| {
                if backup {
                    input = PathBuf::from(format!("{}.backup", input.as_os_str().to_str().unwrap()));
                }

                let cmd = format!(
                    "-overwrite_original_in_place\n-tagsFromFile\n{}\n{}\n-execute\n",
                    input.as_os_str().to_str().unwrap(),
                    output.as_os_str().to_str().unwrap()
                );

                exiftool_stdin.write(cmd.as_bytes()).unwrap();
                let received = rx.recv().unwrap(); // wait for the command to finish
                if received=="{ready}" {
                    println!("{input:?} metadata copied to {output:?}")
                }
            });

    // Close exiftool
    stop_signal.store(true, Ordering::SeqCst);
    exiftool_stdin.write(b"-stay_open\nFalse\n-execute\n").unwrap();
    _stdout_thread.join().unwrap();
    _stderr_thread.join().unwrap();
    exiftool_process.kill()?;

    Ok(())
}
