extern crate sdl2;

use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

mod VDP;

use iz80::AgonMachine;

pub fn main() -> Result<(), String> {
    let (tx_VDP2EZ80, rx_VDP2EZ80): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let (tx_EZ802VDP, rx_EZ802VDP): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let vsync_counter_vdp = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let vsync_counter_ez80 = vsync_counter_vdp.clone();

    println!("Start");

    let cpu_thread = thread::spawn(move || {
        // Prepare the device
        let mut machine = AgonMachine::new(tx_EZ802VDP, rx_VDP2EZ80, vsync_counter_ez80);
        machine.start();
        println!("Cpu thread finished.");
    });

    let mut vdp = VDP::VDP::new(tx_VDP2EZ80, rx_EZ802VDP, vsync_counter_vdp)?;
    vdp.start();

    println!("Quit");

    Ok(())
}