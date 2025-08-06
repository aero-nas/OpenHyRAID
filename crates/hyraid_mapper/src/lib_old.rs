/*!
    Only temporarily kept for reference, won't compile.

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

mod lib_rewrite;
use std::{
    collections::HashMap, hint::unreachable_unchecked, io::Write, path::Path, process::{exit, Command, Stdio}, thread, time::Duration 
};

use hyraid_mdadm;
use hyraid_lvm2;
use hyraid_json;

use hyraid_utils::{
    error_exit,
    unwrap_or_exit,
    unwrap_or_exit_verbose
};

use gpt::{
    self, 
    GptDisk
};
use regex::Regex;
use rand::Rng;

static HYRAID_JSON_PATH: &'static str = "/etc/hyraid.json";

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

fn gen_slices(disks: &[&str]) -> Vec<usize> {
    let mut sizes: Vec<usize> = vec![];
    for disk in disks {
        sizes.push(get_free_space(disk));
    }

    sizes.sort_unstable();

    let min_size: usize = sizes[0];

    let mut slices: Vec<usize> = vec![min_size];

    // skip first element (smallest disk size)
    for size in sizes[1..].iter() { 
        let slice = *size-slices.iter().sum::<usize>(); // Should probably rewrite this line.
        if slice != 0 {
            slices.push(slice)
        };
    }

    slices
}

/// Generates initial partition map
fn init_partition_map(disks: &[&str], slices: &Vec<usize>) -> HashMap<String,Vec<usize>> {
    let mut result: HashMap<String,Vec<usize>> = HashMap::new();

    for disk in disks {
        let size = get_free_space(disk);
        let part = find_range_sum(slices.clone(),size);
        result.insert(disk.to_string(),part);
    }

    result
}

#[derive(Clone)]
struct DiskPartition {
    path: String,
    size: usize
}

struct Disk {
    partitions: Vec<DiskPartition>
}

/// Map-out regular RAID arrays
fn map_raid_arrays(map: &HashMap<String, Vec<usize>>) -> HashMap<String, Vec<String>> {
    let mut disks: Vec<Disk> = vec![];

    for disk in map.keys() {
        let diskpath = std::path::Path::new(disk);
        let mut parts: Vec<DiskPartition> = vec![];
        let gptconfig = unwrap_or_exit!(
            gpt::GptConfig::new()
                .open(diskpath),
            "Failed to open disk."
        );
        for part in gptconfig.partitions().values() {
            parts.push(
                DiskPartition { 
                    path: 
                        "/dev/disk/by-partuuid/".to_string()+&part.part_guid.to_string(), 
                    size: part.sectors_len()
                        .unwrap()
                        .try_into()
                        .unwrap()
                }
            );

            // sort the partitions in the disk from smallest to biggest
            parts.sort_by_key(|k| k.size);
        }
        disks.push(
            Disk { partitions: parts }
        );
    }

    disks.sort_unstable_by_key(|k| k.partitions.len());
    disks.reverse();
    
    let mut groups: HashMap<usize,Vec<String>> = HashMap::new();
    
    for i in 0..(&disks)[0].partitions.len() {
        groups.insert(i,vec![]);
    }

    for (i,v) in &mut groups {
        for disk in disks.iter() {
            if let Some(x) = disk.partitions.get(*i) {
                v.push(x.clone().path);
            }
        }
    }
    
    // wait until all the partitions have been actually created.
    // what. the. fuck.
    for disk in map.keys() {
        let diskpath = std::path::Path::new(disk);
        let disk = unwrap_or_exit!(
            gpt::GptConfig::new()
                .open(diskpath),
            "Failed to open disk."
        );
        for part in disk.partitions().values() {
            let partition = "/dev/disk/by-partuuid/".to_string()+&part.part_guid.to_string();
            while !Path::new(&partition).exists() {
                thread::sleep(Duration::from_millis(100));
            };
        }
    }

    let mut arrays: HashMap<String,Vec<String>> = HashMap::new();

    for group in groups.values() {
        let partitions: Vec<&str> = group
            .iter()
            .map(|s| s.as_str())
            .collect();
        if partitions.len() != 1 {
            // Generate a random name for the RAID array prefixed with "hyraid_md_"
            // to denote that it is part of a HyRAID array and not a regular RAID array
            let devname = &format!("/dev/md/hyraid_md_{}",random_string(10))[..];

            arrays.insert(devname.to_string(),group.to_vec());
        }
    }
    
    arrays
}

fn create_raid_arrays(raid_map: HashMap<String, Vec<String>>, raid_level: usize) {
    if !([0,1,5,6].contains(&raid_level)) {
        error_exit!("Incorrect RAID level. Only RAID0,RAID1,RAID5 and RAID6 is supported.");
    }
    for (devname,group) in raid_map.iter() {
        let partitions: Vec<&str> = group
            .iter()
            .map(|s| s.as_str())
            .collect();
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
                _ => unsafe {unreachable_unchecked()}
            }
        };
                
        unwrap_or_exit_verbose!(
            hyraid_mdadm::create_array(devname,&partitions,level),
            "Error occurred while creating MD array"
        );
    }
}

/// Create LVM logical volume with all of the raid arrays.
fn create_lvm(raid_arrays: &[&str]) -> String {
    let lv_name = format!("hyraid_vg_{}",random_string(16));
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
    
    "/dev/".to_string() + &lv_name + &"/lvol0"
}

/// Create a HyRAID array
/// returns LVM logical volume
pub fn create_array(disks: &[&str],raid_level: usize) -> String {
    for disk in disks {
        ensure_gpt(disk);
        clear_partitions(disk);
    }
    let slices = gen_slices(disks);
    let part_map = init_partition_map(disks,&slices);

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
            disk.write_inplace().unwrap();
        }
    }

    let raid_arrays = map_raid_arrays(&part_map);

    create_raid_arrays(raid_arrays.clone(),raid_level);

    let slice = raid_arrays
        .keys()
        .map(|s| s.as_str())
        .collect::<Vec<&str>>();
    
    let lv_path = create_lvm(&slice);
    let entry = hyraid_json::Entry {
        lvm_vg_path: lv_path.clone(),
        disks: disks
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        raid_arrays: raid_arrays,
        raid_level: raid_level,
        slices: slices
    };

    if Path::new(&lv_path).exists() {
        hyraid_json::create_entry(HYRAID_JSON_PATH,entry);
    }

    lv_path
}

pub fn fail_from_array(
    hyraid_lv: &str,
    disks: &[&str]
) {
    for disk in disks {
        let mut partitions: Vec<String> = vec![];
        let gptdisk: GptDisk<std::fs::File> = unwrap_or_exit!(
            gpt::GptConfig::new()
                .open(disk),
            "Failed to open disk."
        );
        for partition in gptdisk.partitions().values() {
            partitions.push("/dev/disk/by-partuuid/".to_string()+&partition.part_guid.to_string())
        }
        match hyraid_json::read_entries(HYRAID_JSON_PATH).iter().find(|x| x.lvm_vg_path == hyraid_lv) {
            Some(entry) => {
                for part in partitions.clone() {
                    let raid_array = entry.raid_arrays
                        .iter()
                        .find(|x| x.1.contains(&part.to_string()));

                    if let Some(array) = raid_array {
                        for partition in &partitions {
                            if array.1.contains(&partition) {
                                unwrap_or_exit_verbose!(
                                    hyraid_mdadm::fail_from_array(array.0,&[&partition]),
                                    "Failed to mark drive as faulty. mdadm output:"
                                );
                                println!("Marked disk(s) as faulty. on array.");
                            }
                        }
                    }
                }
            },
            None => {
                error_exit!("Error: No such HyRAID LVM volume.");
            }
        };
    }
}

pub fn remove_from_array(
    hyraid_lv: &str,
    disks: &[&str]
) {
    match hyraid_json::read_entries(HYRAID_JSON_PATH).iter().find(|x| x.lvm_vg_path == hyraid_lv) {
        Some(entry) => {
            for disk in disks {
                let mut partitions: Vec<String> = vec![];
                let gptdisk: GptDisk<std::fs::File> = unwrap_or_exit!(
                    gpt::GptConfig::new()
                        .open(disk),
                    "Failed to open disk."
                );
                for partition in gptdisk.partitions().values() {
                    partitions.push("/dev/disk/by-partuuid/".to_string()+&partition.part_guid.to_string())
                }
                
                for part in partitions.clone() {
                    let raid_array = entry.raid_arrays
                        .iter()
                        .find(|x| x.1.contains(&part.to_string()));
                    if let Some(array) = raid_array {
                        for partition in &partitions {
                            if array.1.contains(&partition) {
                                unwrap_or_exit_verbose!(
                                    hyraid_mdadm::remove_from_array(array.0,&[&partition]),
                                    "Failed to mark drive as faulty. mdadm output:"
                                );
                                println!("Removed disk(s) from array.");
                            }
                        }
                    }
                }
            }
        },
            None => {
                error_exit!("Error: No such HyRAID LVM volume.");
            }
        }
}

pub fn add_to_array(
    hyraid_lv: &str,
    disks: &[&str]
) {
    // TODO: Refactor and optimize
    match hyraid_json::read_entries(HYRAID_JSON_PATH).iter().find(|x| x.lvm_vg_path == hyraid_lv) {
        Some(entry) => {
            for disk in disks {
                let diskpath = std::path::Path::new(disk);
                let mut gptdisk = unwrap_or_exit!(
                    gpt::GptConfig::new()
                        .writable(true)
                        .open(diskpath),
                    "Failed to open disk."
                );
                let part = find_range_sum(entry.slices,get_free_space(disk));
                for partition in part {
                    gptdisk.add_partition(
                        "hyraid_partition",
                        partition.try_into().unwrap(),
                        gpt::partition_types::LINUX_FS,
                        0,
                        None
                    ).unwrap();
                    gptdisk.write_inplace().unwrap();
                }
                let mut partitions = vec![];
                for partition in gptdisk.partitions().values() {
                    partitions.push("/dev/disk/by-partuuid/".to_string()+&partition.part_guid.to_string())
                }
                for partition in partitions {
                    let raid_map = map_raid_arrays(&part_map);
                    let raid_map_filter = raid_map
                        .iter()
                        .filter(|x| x.1.contains(&partition))
                        .collect::<HashMap<&String,&Vec<String>>>();
                    for (array,partitions) in raid_map_filter {
                        let _ = hyraid_mdadm::add_to_array(&array,&[&partition]);
                        let level: usize = {
                            match entry.raid_level {
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
                        let _ = hyraid_mdadm::grow_array(&array,level);
                    }
                };
            }
        },
        None => {
            error_exit!("Error: No such HyRAID LVM volume.");
        }
    };
}
