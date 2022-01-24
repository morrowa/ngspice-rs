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
use std::marker::PhantomPinned;
use std::os::raw::{c_char, c_int, c_void};
use std::pin::Pin;
use std::sync::Mutex;

// TODO: impl Error
#[derive(Debug)]
pub enum Error {
    StringEncodingError,
    ParseError,
}

// first: simulator state singleton
// next: interface for passing in a circuit
// next: interface for executing a simulation command
// both synchronous, and async using Futures
// next: vector struct with units

#[derive(Clone, Debug)]
pub struct Simulation {
    pub stdout: String,
    pub stderr: String,
    // TODO:
    // pub vectors:
}

extern "C" fn send_char(str: *mut c_char, _: c_int, ctx: *mut c_void) -> c_int {
    0
}

extern "C" fn controlled_exit(
    _: c_int,
    _: NG_BOOL,
    _: NG_BOOL,
    _: c_int,
    _: *mut c_void,
) -> c_int {
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

    pub fn simulate(circuit: &str, command: &str) -> Result<(), Error> {
        let mut lock = NgSpice::shared().lock().unwrap();
        lock.as_mut().stdout().truncate(0);
        lock.as_mut().stderr().truncate(0);
        // TODO
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
