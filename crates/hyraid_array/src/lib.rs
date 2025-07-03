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


use std::{
    collections::{HashMap, HashSet}, 
    io::Write, 
    process::{Command, Stdio},
    path::Path,
    process::exit,
};

use hyraid_mdadm;

use gpt::{self, GptDisk};
use regex;
use rand::{distr::Alphanumeric, Rng};

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

fn free_space(dev: &'static str) -> usize {
    let diskpath = std::path::Path::new(dev);
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
    for disk in disks {
        sizes.push(free_space(disk));
    }
    sizes.sort();
    let min_size: usize = *sizes.iter().min().unwrap();

    let mut result: HashMap<String,Vec<usize>> = HashMap::new();
    for disk in disks {
        let size: usize = free_space(disk);
        let part = {
            if size > min_size {
                vec![min_size,size-min_size]
            } else {
                vec![min_size,0]
            }
        };
        result.insert(disk.to_string(),part);
    }
    result
}

/// Create regular RAID arrays
fn create_raid(map: &HashMap<String, Vec<usize>>) -> Vec<String> {
    let mut sizes = HashSet::new();
    for disk in map.iter() {
        sizes.insert(disk.1[1]);
    }
    let mut groups: HashMap<usize, Vec<String>> = HashMap::new();
    for size in sizes {
        groups.insert(size,vec![]);
    }
    for disk in map {
        groups.get_mut(&disk.1[1]).unwrap().push(disk.0.to_string());
    }
    let mut arrays: Vec<String> = vec![];
    for group in groups {
        for disk in group.1.iter() {
            let mut partitions: Vec<String> = vec![];
            let diskpath = std::path::Path::new(disk);
            let disk = gpt::GptConfig::new()
                .writable(true)
                .open(diskpath)
                .expect("Failed to open disk");
            for part in disk.partitions() {
                let path = "/dev/disk/by-uuid/".to_string() + &part.1.part_guid.to_string();
                partitions.push(Path::new(&path)
                    .canonicalize()
                    .unwrap()
                    .as_os_str()
                    .to_string_lossy()
                    .to_string()
                );
            }

            let partitions: Vec<&str> = partitions.iter().map(|s| s.as_str()).collect();
            let devname: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect();
            let devname = &format!("/dev/md/hyraid_md_{}",devname)[..];
            let res = hyraid_mdadm::create_array(devname,&partitions,5);
            if res.is_err() {
                println!("{}",res.unwrap_err());
                exit(1);
            } else {
                arrays.push(devname.to_string())
            }
        }
    }
    return arrays;
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
    println!("{:?}",part);
    disk.add_partition(
        "hyraid_partition",
        part.1[0].try_into().unwrap(),
        gpt::partition_types::LINUX_FS,
        0,
        None
    ).unwrap();
    if part.1[1] != 0 {
        disk.add_partition(
            "hyraid_partition",
            part.1[1].try_into().unwrap(),
            gpt::partition_types::LINUX_FS,
            0,
            None
        ).unwrap();
    }
    disk.write().unwrap();
    create_raid(&part_map);
}
}