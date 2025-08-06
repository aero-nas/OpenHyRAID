/* 
    Miscallenous utilities for HyRAID

    Copyright (C) 2025 LIZARD-OFFICIAL-77
    This program is free software; you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation; either version 2 of the License, or
    (at your option) any later version.
    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License along
    with this program; if not, write to the Free Software Foundation, Inc.,
    51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
*/

use nix::unistd::{getuid,ROOT};

pub fn is_root() -> bool {
    return getuid() == ROOT;
}

#[macro_export]
/// Macro to run command and return result.
macro_rules! run_cmd {
    ($cmd:expr) => {
        match $cmd.output() {
            Ok(_) => {
                return Ok(())
            }
            Err(err) => {
                return Err(err.to_string())
            }
        }
    };
}

/// Unwrap result but quit with exit code 1 instead of panicking
#[macro_export]
macro_rules! unwrap_or_exit {
    ($result:expr,$expect:expr) => {{
        match $result {
            Ok(val) => val,
            Err(_) => {
                error_exit!($expect);
            }
        }
    }};
}

/// Unwrap result but quit with exit code 1 instead of panicking, printing the error stored in result.
#[macro_export]
macro_rules! unwrap_or_exit_verbose {
    ($result:expr,$expect:expr) => {{
        match $result {
            Ok(val) => val,
            Err(err) => {
                error_exit!($expect,err);
            }
        }
    }}
}
/// Quit with exit code 1
#[macro_export]
macro_rules! error_exit {
    ($error:expr) => {
        eprintln!("{}",$error);
        exit(1);
    };
    ($description:expr,$error2:expr) => {
        eprintln!("{}",$description);
        eprintln!("{}",$error2);
        exit(1);
    };
}