/*!
    Create, modify or delete HyRAID arrays.
    Main functionality is stored here.

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
    ops::Index,
    process::{exit, Command, Stdio}
};

/// Unwrap result but quit with exit code 1 instead of panicking
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

use hyraid_mdadm;
use hyraid_lvm2;

use gpt::{self, GptDisk};
use regex::Regex;
use rand::Rng;

fn random_string(length: usize) -> String {
    rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Ensures that the partition table of the disk is GPT
fn ensure_gpt(disk: &str) {
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

/// Deletes all partitions on disk
fn clear_partitions(disk: &str) {
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
fn get_free_space(dev: &str) -> usize {
    let diskpath = std::path::Path::new(dev);
    let gptdisk: GptDisk<std::fs::File> = unwrap_or_exit!(
        gpt::GptConfig::new()
            .open(diskpath),
        "Failed to open disk."
    );
    let sectors = gptdisk.find_free_sectors()[0];
    let sectors: usize = (sectors.1-sectors.0).try_into().unwrap();
    
    sectors*512
}

/// Finds range from a vector whose sum is x
fn find_range_sum(vector: Vec<usize>,sum: usize) -> Vec<usize> {
    let mut x = 0;

    while vector[0..x].iter().sum::<usize>() != sum {
        x += 1;
    }

    vector[0..x].to_vec()
}

/// Generates initial partition map
fn init_partition_map(disks: &[&str]) -> HashMap<String,Vec<usize>> {
    let mut sizes: Vec<usize> = vec![];
    for disk in disks {
        sizes.push(get_free_space(disk));
    }

    sizes.sort_unstable();

    let min_size: usize = sizes[0];
    
    let mut result: HashMap<String,Vec<usize>> = HashMap::new();

    let mut slices: Vec<usize> = vec![min_size];

    // skip first element (smallest disk size)
    for size in sizes[1..].iter() { 
        let slice = *size-slices.iter().sum::<usize>(); // Should probably rewrite this line.
        if slice != 0 {
            slices.push(slice)
        };
    }

    for disk in disks {
        let size = get_free_space(disk);
        let part = find_range_sum(slices.clone(),size);
        result.insert(disk.to_string(),part);
    }

    result
}

/// Create regular RAID arrays
fn create_raid_arrays(map: &HashMap<String, Vec<usize>>,raid_level: usize) -> Vec<String> {
    if !([0,1,5,6].contains(&raid_level)) {
        error_exit!("Incorrect RAID level. Only RAID0,RAID1,RAID5 and RAID6 is supported.");
    }

    let mut unique_sizes = HashSet::<usize>::new();

    for disk in map.keys() {
        let diskpath = std::path::Path::new(disk);
        let disk = unwrap_or_exit!(
            gpt::GptConfig::new()
                .open(diskpath),
            "Failed to open disk."
        );
        for part in disk.partitions().values() {
            unique_sizes.insert(
                part.sectors_len()
                    .unwrap()
                    .try_into()
                    .unwrap()
            );
        }
    }

    let mut groups: HashMap<usize, Vec<String>> = HashMap::new();

    for size in unique_sizes {
        groups.insert(size,vec![]);
    }

    for disk in map.keys() {
        let diskpath = std::path::Path::new(disk);
        let disk = unwrap_or_exit!(
            gpt::GptConfig::new()
                .open(diskpath),
            "Failed to open disk."
        );

        for part in disk.partitions().values() {
            let partition = "/dev/disk/by-partuuid/".to_string()+&part.part_guid.to_string();
            let sectors: usize = part.sectors_len().unwrap() as usize;
            groups.get_mut(&sectors).unwrap().push(partition);
        }
    }

    let mut arrays: Vec<String> = vec![];
    println!("{:?}",groups);
    for group in groups.values() {
        let partitions: Vec<&str> = group
            .iter()
            .map(|s| s.as_str())
            .collect();
        // Generate a random name for the RAID array prefixed with "hyraid_md_"
        // to denote that it is part of a HyRAID array and not a regular RAID array
        
        let devname = &format!("/dev/md/hyraid_md_{}",random_string(10))[..];
        
        let level: usize = {
            match raid_level {
                0 => 0,
                1 => 1,
                5 => {
                    if partitions.len() < 3 {
                        1 // RAID1
                    } else {
                        5 // RAID5
                    }
                },
                6 => {
                    if partitions.len() < 3 {
                        1 // RAID1
                    } else {
                        6 // RAID6
                    }
                },
                
                // this is deadass needed?
                _ => unreachable!()
            }
        };
        
        unwrap_or_exit_verbose!(
            hyraid_mdadm::create_array(devname,&partitions,level),
            "Error occurred while creating MD array"
        );
        arrays.push(devname.to_string())
    }
    
    arrays
}

/// Create LVM logical volume with all of the raid arrays.
fn create_lvm(raid_arrays: &[&str]) -> String {
    let lv_name = format!("hyraid_lv_{}",random_string(16));
    unwrap_or_exit_verbose!(
        hyraid_lvm2::lvm_pv_create(raid_arrays),
        "Error occured while setting up LVM"
    );
    unwrap_or_exit_verbose!(
        hyraid_lvm2::lvm_vg_create(&lv_name[..],raid_arrays),
        "Error occured while setting up LVM"
    );
    unwrap_or_exit_verbose!(
        hyraid_lvm2::lvm_lv_create(&lv_name[..],raid_arrays,hyraid_lvm2::SizeFormat::EXTENTS,"100%FREE"),
        "Error occured while setting up LVM"
    );
    
    lv_name
}

/// Create a HyRAID array
/// returns LVM logical volume
pub fn create_array(disks: &[&str],raid_level: usize) -> String {
    for disk in disks {
        ensure_gpt(disk);
        clear_partitions(disk);
    }
    let part_map = init_partition_map(disks);
    println!("{:?}",part_map);
    for part in part_map.iter() {
        let diskpath = std::path::Path::new(part.0);
        let mut disk = unwrap_or_exit!(
            gpt::GptConfig::new()
                .writable(true)
                .open(diskpath),
            "Failed to open disk."
        );
        for partition in part.1 {
            disk.add_partition(
                "hyraid_partition",
                (*partition).try_into().unwrap(),
                gpt::partition_types::LINUX_FS,
                0,
                None
            ).unwrap();
        }
    
        disk.write().unwrap();
    }
    let raid_arrays = create_raid_arrays(&part_map,raid_level);

    let slice = raid_arrays
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<&str>>();
    
    create_lvm(&slice)
}