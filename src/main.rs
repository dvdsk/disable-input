#![feature(unix_chown)]

use std::os::unix::fs::chown;
use std::time::Duration;
use std::{env, thread};

mod input;
mod setuid;

const ROOT: u32 = 0;
const HELP: &str = "Usage: call with --lock to disable keyboard and mouse. \n
call with --unlock to re-enable them.\n\n
        - Requires sudo on its first run, will rerun with sudo when not provided. \n\
        Options:\n    --help, -h    Print this help message\n";

fn main() {
    if let Some(arg) = env::args().nth(1) {
        if arg == "--help" || arg == "-h" {
            println!("{HELP}");
            return;
        }
    }

    sudo::escalate_if_needed()
        .expect("sudo failed, you may also call this with sudo in front of it");

    let was_set = setuid::is_set();

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
    let mouse = devices
        .iter()
        .find(|e| e.name.starts_with("USB Optical Mouse"))
        .unwrap();
    let keyboard = devices
        .iter()
        .find(|e| e.name.starts_with("HID 046a:010d"))
        .unwrap();
    let locked_mouse = mouse.clone().lock().unwrap();
    let locked_keyboard = keyboard.clone().lock().unwrap();

    println!("unlocking in 60 seconds");
    thread::sleep(Duration::from_secs(60));

    locked_mouse.unlock();
    locked_keyboard.unlock();
}
