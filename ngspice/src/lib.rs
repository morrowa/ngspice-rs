// Copyright 2022 Andrew Morrow.
// lib.rs
// ngspice
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

use ngspice_sys::*;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fmt::{self, Formatter};
use std::marker::PhantomPinned;
use std::os::raw::{c_char, c_int, c_void};
use std::pin::Pin;
use std::ptr;
use std::sync::Mutex;

#[derive(Debug)]
pub enum Error {
    /// A string argument could not be converted to null-terminated UTF-8.
    InvalidStringEncoding,
    /// ngSPICE was unable to parse the circuit. The contained String holds error logs.
    InvalidCircuit(String),
    /// ngSPICE returned an unknown error. The contained String holds error logs.
    Unknown(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidStringEncoding => {
                f.write_str("invalid string encoding; all strings must be UTF-8 with no null bytes")
            }
            Error::InvalidCircuit(msg) => f.write_fmt(format_args!(
                "error parsing circuit; ngSPICE logs follow:\n{}",
                msg
            )),
            Error::Unknown(msg) => {
                f.write_fmt(format_args!("unknown error; ngSPICE logs follow:\n{}", msg))
            }
        }
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug)]
pub enum DataType {
    Unknown,
    Time,
    Frequency,
    Voltage,
    Current,
    // TODO: the rest
}

impl From<simulation_types::Type> for DataType {
    fn from(x: simulation_types::Type) -> Self {
        match x {
            simulation_types::SV_TIME => DataType::Time,
            simulation_types::SV_FREQUENCY => DataType::Frequency,
            simulation_types::SV_VOLTAGE => DataType::Voltage,
            simulation_types::SV_CURRENT => DataType::Current,
            // TODO: the rest
            _ => DataType::Unknown,
        }
    }
}

#[derive(Clone, Debug)]
pub enum VectorValues {
    Real(Vec<f64>),
    Complex(Vec<num_complex::Complex64>),
}

#[derive(Clone, Debug)]
pub struct VectorInfo {
    pub datatype: DataType,
    pub values: VectorValues,
}

/// Represents the results of a single ngSPICE simulation (aka an ngSPICE plot).
#[derive(Clone, Debug, Default)]
pub struct Simulation {
    /// ngSPICE log output to stdout.
    pub stdout: String,
    /// ngSPICE log output to stderr.
    pub stderr: String,
    /// All simulation output vectors by name.
    pub vectors: HashMap<String, VectorInfo>,
}

impl Simulation {
    unsafe fn insert_vecinfo(&mut self, v: *const vector_info) {
        let name = CStr::from_ptr((*v).v_name);
        let name = name
            .to_str()
            .expect("ngSPICE sent non-UTF8 vector name")
            .to_owned();
        let datatype = DataType::from((*v).v_type as u32);
        let len: usize = (*v).v_length as usize;
        let values: VectorValues = if (*v).v_realdata != ptr::null_mut() {
            let ary = std::slice::from_raw_parts((*v).v_realdata, len).to_owned();
            VectorValues::Real(ary)
        } else {
            assert_ne!(
                (*v).v_compdata,
                ptr::null_mut(),
                "ngSPICE vector_info must have either real or complex values"
            );
            // as of ngspice-35, the ngcomplex struct is memory-layout compatible with num_complex::Complex64
            // if that changes, this explodes
            let ary: &[ngcomplex_t] = std::slice::from_raw_parts((*v).v_compdata, len);
            let ary: &[num_complex::Complex64] = std::mem::transmute(ary);
            let ary = ary.to_owned();
            VectorValues::Complex(ary)
        };
        let vecinfo = VectorInfo { datatype, values };
        self.vectors.insert(name, vecinfo);
    }
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

/// Interface to ngSPICE.
#[derive(Debug)]
pub struct NgSpice {
    stdout: String,
    stderr: String,
    _pin: PhantomPinned,
}

impl NgSpice {
    fn shared() -> &'static Mutex<Pin<Box<NgSpice>>> {
        NGSPICE.get_or_init(|| {
            let mut sim = Box::pin(NgSpice {
                stdout: String::new(),
                stderr: String::new(),
                _pin: PhantomPinned,
            });
            unsafe {
                ngSpice_Init(
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

    /// Parses a new circuit and executes a simulation command, returning the complete results.
    ///
    /// This function will block until the simulation completes. It may safely be called from any
    /// thread, but only one simulation will be executed at a time.
    ///
    /// # Arguments
    ///
    /// * `circuit` - An ngSPICE circuit listing. Must be self-contained
    ///   (i.e. may not use the `.include` command).
    ///
    /// * `command` - An ngSPICE simulation command like `ac` or `tran`.
    ///
    /// # Panics
    ///
    /// This function will panic if ngSPICE encounters an unrecoverable error.
    ///
    /// # Errors
    ///
    /// If any argument cannot be converted to a null-terminated UTF-8 string, this function will
    /// return an error.
    ///
    /// If ngSPICE cannot parse the circuit or the command, this function will return an error.
    pub fn simulate(circuit: &str, command: &str) -> Result<Simulation, Error> {
        NgSpice::check_circuit(circuit)?;
        NgSpice::check_command(command)?;
        // We intentionally panic if the Mutex is poisoned, because ngSPICE cannot recover
        let mut handle = NgSpice::shared().lock().unwrap();
        handle.as_mut().stdout().truncate(0);
        handle.as_mut().stderr().truncate(0);
        handle.as_mut().load_circuit(circuit)?;
        handle.as_mut().command(command)?;
        let mut sim = Simulation::default();
        unsafe {
            let mut vec_name = ngSpice_AllVecs(ngSpice_CurPlot()) as *const *mut c_char;
            while *vec_name != ptr::null_mut() {
                sim.insert_vecinfo(ngGet_Vec_Info(*vec_name));
                vec_name = vec_name.add(1);
            }
        }
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
                Err(Error::InvalidCircuit(self.stderr().clone()))
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
                Err(Error::Unknown(self.stderr().clone()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, NgSpice};

    #[test]
    fn it_works() -> Result<(), Error> {
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
        assert!(sim.vectors.len() > 0);
        Ok(())
    }
}
