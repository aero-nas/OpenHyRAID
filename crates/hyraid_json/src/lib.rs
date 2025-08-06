/*!
    JSON file for HyRAID data
    
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

use std::{
    fs, io::Write, path::Path
};
use hyraid_types::HyraidArray;

fn ensure_json_file_exists(path: &str) {
    if !(Path::new(path).exists()) {
        let mut file = fs::File::create(path).unwrap();
        file.write(b"[]").unwrap();
    };
}

pub fn read_arrays(path: &str) -> Vec<HyraidArray> {
    ensure_json_file_exists(path);

    let data = fs::read_to_string(path).unwrap();
    let entries: Vec<HyraidArray> = serde_json::from_str(&data).unwrap();

    entries
}

/// Replaces an entry with the given entry
pub fn modify(path: &str,name: String,hyraid_array: HyraidArray) {
    ensure_json_file_exists(path);

    let entries = read_arrays(path);
    let entries: Vec<&HyraidArray> = entries
        .iter()
        .map(|x| {
            if x.name == name {
                &hyraid_array
            } else {
                x
            }
        }
    ).collect();
    
    
    let json = serde_json::to_string_pretty(&entries).unwrap();
    fs::write(path,json).unwrap();
}

/// Add array entry to json file
pub fn write_array(path: &str, hyraid_array: HyraidArray) {
    ensure_json_file_exists(path);

    let mut entries = read_arrays(path);
    entries.push(hyraid_array);

    let json = serde_json::to_string_pretty(&entries).unwrap();
    fs::write(path,json).unwrap();
}

