/*
    Create, modify or delete HyRAID arrays.

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


use std::{collections::HashMap, io::Write, process::{Command, Stdio}, vec};

use lsblk::blockdevs::BlockDevice;
use regex;
use gpt::{self, GptDisk};

/// Ensures that the partition table of the disk is GPT
fn ensure_gpt(disk: &'static str) {
    let cmd = Command::new("sfdisk")
        .arg("-d")
        .arg(disk)
        .output()
        .expect(&format!("Incorrect device: {}",disk));
    let stdout = String::from_utf8(cmd.stdout).unwrap();
    let regex = regex::Regex::new("label: (?<table>.+)").unwrap();
    let table = regex.captures(&stdout).unwrap()
        .name("table")
        .unwrap()
        .as_str();

    if table == "gpt" {
        return; // disk is already gpt
    }

    let mut process = Command::new("sfdisk")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .arg(disk)
        .spawn()
        .expect(&format!("Incorrect disk: {}",disk));

    process.stdin
        .as_mut()
        .unwrap()
        .write_all(b"label: gpt\n\twrite\n")
        .unwrap(); // this is kind of a hack but works
    process.wait().unwrap();
    println!("Converted disk {} to GPT partition table",disk);
}

/// Deletes all partitions on disk
fn clear_partitions(disk: &'static str) {
    let diskpath = std::path::Path::new(disk);
    let mut gptdisk: GptDisk<std::fs::File> = gpt::GptConfig::new()
        .writable(true)
        .open(diskpath)
        .expect("Failed to open disk");
    let parts = gptdisk.partitions().clone();
    for part in parts {
        gptdisk.remove_partition(part.0);
    }
    gptdisk.write().unwrap();
}

fn free_space(dev: &BlockDevice) -> usize {
    let diskpath = std::path::Path::new(&dev.fullname);
    let gptdisk: GptDisk<std::fs::File> = gpt::GptConfig::new()
        .writable(true)
        .open(diskpath)
        .expect("Failed to open disk");
    let sectors = gptdisk.find_free_sectors()[0];
    let sectors: usize = (sectors.1-sectors.0).try_into().unwrap();
    return sectors*512
}

/// Generates partitions as hashmap
// This is the main functionality of Synology SHR.
fn gen_partitions(disks: &[&'static str]) -> HashMap<String,Vec<usize>> {
    let mut sizes: Vec<usize> = vec![];
    let mut disks_array: Vec<BlockDevice> = vec![]; // disks to be used in (HyRAID) array
    for blockdev in BlockDevice::list().unwrap().iter() {
        if disks.contains(
            &&blockdev.fullname.clone()
            .into_os_string()
            .into_string()
            .unwrap()[..]
        ){
            sizes.push(free_space(blockdev));
            disks_array.push(blockdev.clone())
        }
    }
    sizes.sort();
    let min_size: usize = *sizes.iter().min().unwrap();

    let mut result: HashMap<String,Vec<usize>> = HashMap::new();
    for disk in disks_array {
        let size: usize = free_space(&disk);
        let part = {
            if size > min_size {
                vec![min_size,size-min_size]
            } else {
                vec![min_size,0]
            }
        };
        result.insert(disk.fullname
            .into_os_string()
            .into_string()
            .unwrap(),part);
    }
    result
}

pub fn create_array(disks: &[&'static str]) {
    for disk in disks {
        ensure_gpt(disk);
        clear_partitions(disk);
    }
    let part_map = gen_partitions(disks);
    for part in part_map.iter() {
        let diskpath = std::path::Path::new(part.0);
        let mut disk = gpt::GptConfig::new()
            .writable(true)
            .open(diskpath)
            .expect("Failed to open disk");
        disk.add_partition(
            "hyraid_partition",
            (part.1[0]).try_into().unwrap(),
            gpt::partition_types::LINUX_FS,
            0,
            None
        ).unwrap();
        if part.1[1] != 0 {
            disk.add_partition(
                "hyraid_partition",
                (part.1[1]).try_into().unwrap(),
                gpt::partition_types::LINUX_FS,
                0,
                None
            ).unwrap();
        }
        disk.write().unwrap();
    }
}