use std::error::Error;
use std::path::PathBuf;
use std::process::{ChildStdin, Stdio};
use std::thread::JoinHandle;
use std::io::{BufReader, BufRead, Write};
use std::process::Child;
use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::thread;
use std::boxed::Box;

pub struct ExifTool {
    exiftool_process: Child,
    stop_signal: Arc<AtomicBool>,
    stdin: ChildStdin,
    stdout_thread: Option<JoinHandle<()>>,
    stderr_thread: Option<JoinHandle<()>>,
    stdout_receiver: Receiver<String>,
}

impl ExifTool {
    pub fn new() -> Result<Self, Box<dyn Error>> {

        // spawn exiftool process
        let exiftool_process_result = std::process::Command::new("exiftool")
            .args(["-stay_open", "true", "-@", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        if exiftool_process_result.is_err() {
            println!("This build of rimage requires exiftool (https://exiftool.org) available in the working directory.");
        }
        let mut process = exiftool_process_result?;

        // register stop signal for communication threads
        let stop_signal = Arc::new(AtomicBool::new(false));

        // Grab stdin so we can pipe commands to ExifTool
        let stdin = process
            .stdin
            .take().ok_or("Unable to take stdin")?;

        // Grab stdout/err
        let (stdout_transmitter, rx) = mpsc::channel();
        let stdout = process
            .stdout
            .take().ok_or("Unable to take stdout")?;
        let stderr = process
            .stderr
            .take().ok_or("Unable to take stderr")?;

        // Create a separate thread to loop over stdout
        let stdout_thread = Some(thread::spawn({ let stop_stdout = stop_signal.clone();
            move || {
                let stdout_lines = BufReader::new(stdout).lines();

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
            }}));

        // Create a separate thread to loop over stderr
        // Anything which comes through stderr will be send to console.
        let stderr_thread = Some(thread::spawn({ let stop_stderr = stop_signal.clone();
            move || {
                let stderr_lines = BufReader::new(stderr).lines();
                for line in stderr_lines {
                    let line = line.unwrap();
                    println!("exiftool: {}", line);

                    if stop_stderr.load(Ordering::SeqCst) {
                        return
                    }
                }
            }
        }));

        Ok(ExifTool{
            exiftool_process: process,
            stop_signal,
            stdout_thread,
            stderr_thread,
            stdin,
            stdout_receiver: rx,
        })
    }

    pub fn copy_metadata(
        &mut self,
        iterator: impl IntoIterator<Item = (PathBuf, PathBuf)>,
        backup: bool,
        ) -> Result<(), Box<dyn Error>> {

        // Loop over target files
        iterator.into_iter()
            .for_each(|(mut input, output): (PathBuf, PathBuf)| {
                if backup {
                    input = PathBuf::from(format!("{}.backup", input.as_os_str().to_str().unwrap()));
                }

                let cmd = format!(
                    "-overwrite_original_in_place\n-tagsFromFile\n{}\n{}\n-execute\n",
                    input.as_os_str().to_str().unwrap(),
                    output.as_os_str().to_str().unwrap()
                );

                self.stdin.write(cmd.as_bytes()).unwrap();
                let received = self.stdout_receiver.recv().unwrap(); // wait for the command to finish
                if received == "{ready}" {
                    println!("{input:?} metadata copied to {output:?}")
                }
            });

        Ok(())
    }
}

impl Drop for ExifTool {
    fn drop(&mut self)
    {
        // signal exiftool and stdout/err readers to stop
        self.stop_signal.store(true, Ordering::SeqCst);
        self.stdin.write(b"-stay_open\nFalse\n-execute\n").unwrap();

        // wait for everything to shut down
        if self.stdout_thread.is_some() {
           self.stdout_thread.take().unwrap().join().unwrap();
        }
        if self.stderr_thread.is_some() {
            self.stderr_thread.take().unwrap().join().unwrap();
        }

        // kill exiftool process if necessary
        self.exiftool_process.kill().unwrap();
    }
}