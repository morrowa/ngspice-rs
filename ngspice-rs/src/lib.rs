// Copyright 2022 Andrew Morrow.
// lib.rs
// ngspice-rs-sys
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use ngspice_rs_sys::*;
use once_cell::sync::OnceCell;
use std::ffi::{CStr, CString};
use std::fmt::{self, Formatter};
use std::marker::PhantomPinned;
use std::os::raw::{c_char, c_int, c_void};
use std::pin::Pin;
use std::ptr;
use std::sync::Mutex;

#[derive(Debug)]
pub enum Error {
    InvalidStringEncoding,
    InvalidCircuit,
    Unknown,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidStringEncoding => {
                f.write_str("invalid string encoding; all strings must be UTF-8 with no null bytes")
            }
            Error::InvalidCircuit => f.write_str("error parsing circuit; see ngSPICE logs"),
            Error::Unknown => f.write_str("unknown error"),
        }
    }
}

impl std::error::Error for Error {}

// first: simulator state singleton
// next: interface for passing in a circuit
// next: interface for executing a simulation command
// both synchronous, and async using Futures
// next: vector struct with units

#[derive(Clone, Debug, Default)]
pub struct Simulation {
    pub stdout: String,
    pub stderr: String,
    // TODO:
    // pub vectors:
}

extern "C" fn send_char(str: *mut c_char, _: c_int, ctx: *mut c_void) -> c_int {
    let ctx = ctx as *mut NgSpice;
    unsafe {
        let str = CStr::from_ptr(str)
            .to_str()
            .expect("non-UTF8 output from ngSPICE");
        if let Some(x) = str.strip_prefix("stderr ") {
            (*ctx).stderr.push_str(x);
            (*ctx).stderr.push('\n');
        } else if let Some(x) = str.strip_prefix("stdout ") {
            (*ctx).stdout.push_str(x);
            (*ctx).stdout.push('\n');
        } else {
            (*ctx).stdout.push_str(str);
            (*ctx).stdout.push('\n');
        }
    }
    0
}

extern "C" fn controlled_exit(_: c_int, _: NG_BOOL, _: NG_BOOL, _: c_int, _: *mut c_void) -> c_int {
    panic!("fatal ngspice error");
}

static NGSPICE: OnceCell<Mutex<Pin<Box<NgSpice>>>> = OnceCell::new();

#[derive(Debug)]
pub struct NgSpice {
    stdout: String,
    stderr: String,
    _pin: PhantomPinned,
}

impl NgSpice {
    // it seems like this has to return &mut to prevent threading bugs.
    // except, there's nothing to stop someone calling this from any thread
    // I can only think of one method that can be called from any thread: Mutex::lock()
    // so, if I want to have an async interface that uses Futures, I have to force the caller to
    // hold the lock until the future finishes
    // alternatively, I could only offer a sync interface at first, and add async later
    // but even then, it would be best to have a mutex
    fn shared() -> &'static Mutex<Pin<Box<NgSpice>>> {
        NGSPICE.get_or_init(|| {
            let mut sim = Box::pin(NgSpice {
                stdout: String::new(),
                stderr: String::new(),
                _pin: PhantomPinned,
            });
            unsafe {
                ngspice_rs_sys::ngSpice_Init(
                    Some(send_char),
                    None,
                    Some(controlled_exit),
                    None,
                    None,
                    None,
                    sim.as_mut().get_unchecked_mut() as *mut _ as *mut c_void,
                );
            }
            Mutex::new(sim)
        })
    }

    fn stdout(self: Pin<&mut Self>) -> &mut String {
        unsafe { &mut self.get_unchecked_mut().stdout }
    }

    fn stderr(self: Pin<&mut Self>) -> &mut String {
        unsafe { &mut self.get_unchecked_mut().stderr }
    }

    pub fn simulate(circuit: &str, command: &str) -> Result<Simulation, Error> {
        NgSpice::check_circuit(circuit)?;
        NgSpice::check_command(command)?;
        let mut handle = NgSpice::shared().lock().unwrap();
        handle.as_mut().stdout().truncate(0);
        handle.as_mut().stderr().truncate(0);
        handle.as_mut().load_circuit(circuit)?;
        handle.as_mut().command(command)?;
        // TODO gather the result vectors
        let mut sim = Simulation::default();
        std::mem::swap(handle.as_mut().stdout(), &mut sim.stdout);
        std::mem::swap(handle.as_mut().stderr(), &mut sim.stderr);
        Ok(sim)
    }

    fn check_circuit(circuit: &str) -> Result<(), Error> {
        if circuit.as_bytes().contains(&0) {
            return Err(Error::InvalidStringEncoding);
        }
        // TODO: make sure the circuit doesn't contain any commands that could screw up our state
        // e.g. anything that would start a background process
        // TODO: other checks?
        // e.g. check for .end
        Ok(())
    }

    /// You must run check_circuit() first or else this may panic
    fn load_circuit(self: Pin<&mut Self>, circuit: &str) -> Result<(), Error> {
        // need a null-terminated array of null-terminated lines
        let lines: Vec<CString> = circuit
            .lines()
            .map(|l| CString::new(l).expect("illegal char in circuit"))
            .collect();
        let mut clines: Vec<*const c_char> = lines.iter().map(|l| l.as_ptr()).collect();
        clines.push(ptr::null());
        unsafe {
            // ngSPICE does not actually mutate the strings, but it fails to mark its pointers const
            if ngSpice_Circ(clines.as_mut_ptr() as *mut *mut c_char) == 0 {
                Ok(())
            } else {
                Err(Error::InvalidCircuit)
            }
        }
    }

    fn check_command(cmd: &str) -> Result<(), Error> {
        if cmd.as_bytes().contains(&0) {
            return Err(Error::InvalidStringEncoding);
        }

        // TODO: prevent quit?
        Ok(())
    }

    /// You must run check_command first or else this may panic
    fn command(self: Pin<&mut Self>, cmd: &str) -> Result<(), Error> {
        let cmd = CString::new(cmd).expect("illegal char in command");
        unsafe {
            // ngSPICE does not actually mutate the strings, but it fails to mark its pointers const
            if ngSpice_Command(cmd.as_ptr() as *mut c_char) == 0 {
                Ok(())
            } else {
                Err(Error::Unknown)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, NgSpice};

    #[test]
    fn it_works() -> Result<(), Error> {
        // let result = 2 + 2;
        // assert_eq!(result, 4);
        let circuit = ".title Thing
V2 refv GND dc(3.3)
V1 vin GND sin(0 17.4 60)
R3 meas GND 10k
R1 vin meas 60.4k
R4 refv meas 10k
.end";
        let cmd = "tran 100u 0.17s";
        let sim = NgSpice::simulate(circuit, cmd)?;
        assert!(sim.stdout.len() > 0);
        assert!(sim.stderr.len() > 0);
        Ok(())
    }
}
