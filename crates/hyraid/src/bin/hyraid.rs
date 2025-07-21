/*
    Open source implementation of Synology SHR.

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

use hyraid_utils::is_root;
use std::process;

fn main() {
    if !is_root() {
        println!("HyRAID must be run as root. Quitting.");
        process::exit(1);
    }
    println!("THIS PROGRAM IS IN W.I.P. RUNNING IT WILL RESULT IN UNDEFINED BEHAVIOUR!!!");
}