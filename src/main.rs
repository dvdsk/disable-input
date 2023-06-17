#![feature(unix_chown)]
#![feature(iter_intersperse)]

use clap::{Parser, Subcommand};

use std::os::unix::fs::chown;
use std::thread;
use std::time::Duration;

use crate::input::Device;

mod input;
mod setuid;

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// list the devices that can be locked
    List,
    /// lock one or more devices
    Lock {
        /// duration to lock in seconds
        seconds: u64,
        /// names of the devices to lock (use list to get those),
        /// can be passed multiple times to lock multiple devices
        to_lock: Vec<String>,
    },
}

/// Disables keyboard and mouse input for some time
/// Requires sudo on its first run, will rerun with sudo when not provided.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    let args = Args::parse();

    sudo::escalate_if_needed()
        .expect("sudo failed, you may also call this with sudo in front of it");

    let was_set = setuid::is_set();

    const ROOT: u32 = 0;
    let path = std::env::current_exe().unwrap();
    chown(path, Some(ROOT), Some(ROOT)).unwrap();
    setuid::set();

    if !was_set {
        let path = std::env::args().next().unwrap();
        println!(
            "- setuid bit and permissions set and \n    {path}\n\
             now owned by root\n\
            - next time you can run without sudo!\n\
            - next time lock/unlock will happen instandly"
        );
    }

    let devices = input::list().unwrap();
    let (seconds, to_lock) = match args.command {
        Commands::List => {
            let list: String = devices
                .iter()
                .map(|d| d.name.as_str())
                .intersperse("\n")
                .collect();
            println!("devices:\n{list}");
            return;
        }
        Commands::Lock { seconds, to_lock } => (seconds, to_lock),
    };

    let mut to_lock: Vec<_> = devices
        .into_iter()
        .filter(|d| to_lock.contains(&d.name))
        .collect();
    to_lock.dedup_by_key(|d| d.event_path.clone());

    let _locked: Vec<_> = to_lock
        .into_iter()
        .map(Device::lock)
        .collect::<Result<_, _>>()
        .unwrap();

    println!("unlocking in {seconds} seconds");
    thread::sleep(Duration::from_secs(seconds));

    // locked is dropped here unlocking all devices
}
