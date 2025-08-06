/*!
    Additional functionality not present in the gpt crate
    such as support for special sector sizes other than 4096/512

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

use hyraid_utils::{
    unwrap_or_exit,
    error_exit
};
use std::{
    io::Write, 
    fs,
    process::{exit, Command, Stdio},
    thread, 
    time::Duration,
    path::Path
};
use regex::Regex;
use gpt::{
    self, partition::Partition, GptDisk
};

pub fn get_path_of_partition(partition: &Partition) -> String {
    "/dev/disk/by-partuuid/".to_string()+&partition.part_guid.to_string()
}

/// Waits until partition actually exists
pub fn validate_partition(partition: Partition) {
    while !(Path::new(&get_path_of_partition(&partition)).exists()) {
        println!("hanged. {}",get_path_of_partition(&partition));
        thread::sleep(Duration::from_millis(100));
    };
}

/// Ensures that the partition table of the disk is GPT
pub fn ensure_gpt(disk: &str) {
    let cmd = unwrap_or_exit!(
        Command::new("sfdisk")
            .arg("-d")
            .arg(disk)
            .output(),
        format!("Incorrect device: {}",disk)
    );

    let stdout = String::from_utf8(cmd.stdout).unwrap();
    let regex = Regex::new("label: (?<table>.+)").unwrap();
    let table = regex.captures(&stdout).unwrap()
        .name("table")
        .unwrap()
        .as_str();

    if table == "gpt" {
        return; // disk is already gpt
    }
    
    let mut process = unwrap_or_exit!(
        Command::new("sfdisk")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .arg(disk)
            .spawn(),
        format!("Incorrect disk: {}",disk)
    );

    process.stdin
        .as_mut()
        .unwrap()
        .write_all(b"label: gpt\n\twrite\n")
        .unwrap(); // this is kind of a hack but works

    process.wait().unwrap();
    println!("Converted disk {} to GPT partition table",disk);
}

pub fn get_sector_size(disk: &str) -> usize {
    // To account for /dev/disk/by-*/*
    let dev_path = fs::canonicalize(disk).unwrap().to_string_lossy().to_string();

    let dev_path = dev_path.trim_start_matches("/dev/").trim_end_matches("/");
    let sector_size = fs::read_to_string(format!("/sys/block/{}/queue/logical_block_size", dev_path)).unwrap();

    sector_size
        .trim()
        .parse::<usize>()
        .unwrap()
}

/// Deletes all partitions on disk
pub fn clear_partitions(disk: &str) {
    let diskpath = std::path::Path::new(disk);
    let mut gptdisk: GptDisk<std::fs::File> = unwrap_or_exit!(
        gpt::GptConfig::new()
            .writable(true)
            .open(diskpath),
        "Failed to open disk."
    );
    let parts = gptdisk.partitions().clone();
    for part in parts {
        gptdisk.remove_partition(part.0);
    }
    gptdisk.write().unwrap();
}

/// Gets free space on a disk in bytes
pub fn get_free_space(dev: &str) -> usize {
    let diskpath = std::path::Path::new(dev);
    let gptdisk: GptDisk<std::fs::File> = unwrap_or_exit!(
        gpt::GptConfig::new()
            .open(diskpath),
        "Failed to open disk."
    );
    let sectors = gptdisk.find_free_sectors()[0];

    // what the fuck is this bullshit
    let sectors: usize = (sectors.1-sectors.0).try_into().unwrap();
    
    sectors*get_sector_size(dev)
}