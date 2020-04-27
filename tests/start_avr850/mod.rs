/*
 *  arcamclient —  A gtk-rs based Rust application for controlling Arcam amplifiers.
 *
 *  Copyright © 2020  Russel Winder
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

use std::process;
use std::thread;
use std::time;

use ctor::{ctor, dtor};

use rand;
use rand::Rng;

static mut MOCK_AVR850: Option<process::Child> = None;
pub static mut portNumber: u16 = 0;

#[ctor]
fn start_mock_avr850() {
    let mut rng = rand::thread_rng();
    unsafe {
        portNumber = rng.gen_range(50001, 65535);
        eprintln!("==== start_mockavr850: starting the process using port {}", portNumber);
    }
    match process::Command::new("cargo")
        .args(&["run", "--bin", "mock_avr850", unsafe { &portNumber.to_string() }])
        .spawn() {
        Ok(m) => {
            unsafe { MOCK_AVR850 = Some(m); }
            // The server needs a moment to settle before things will work.
            thread::sleep(time::Duration::from_millis(100));
        },
        Err(e) => panic!("====  start_mockavr850: failed to start MOCK_AVR850 – {}", e),
    }
}

#[dtor]
fn terminate_mock_avr850() {
    // Do not do IO here to avoid SIGILL  eprintln!("start_mockavr850: terminating the process.");
    unsafe {
        match &mut MOCK_AVR850 {
            Some(m) => {
                match m.kill() {
                    Ok(_) => {
                        match m.wait() {
                            Ok(_) => {},
                            Err(e) => eprintln!("====  start_avr850: failed to wait on mock_avr850 process: {:?}", e),
                        }
                    },
                    Err(e) => eprintln!("====  start_avr850: failed to terminate mock_avr850 process: {:?}", e),
                }
            },
            None => {},
        }
    }
}
