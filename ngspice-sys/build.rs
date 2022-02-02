// Copyright 2022 Andrew Morrow.
// build.rs
// ngspice-sys
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

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=ngspice");
    // TODO: don't hard-code these paths
    println!("cargo:rustc-link-search=/usr/local/ngspice/lib");
    println!("cargo:rerun-if-changed=wrapper.h");
    let bindings = bindgen::builder()
        .constified_enum_module("simulation_types")
        // TODO: don't hard-code these paths
        .clang_arg("-I/usr/local/ngspice/include")
        .header("wrapper.h")
        .generate()
        .expect("Unable to generate ngSPICE bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Unable to write ngSPICE bindings");
}
