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
    collections::HashMap, 
    path::Path, 
    process::exit
};

use hyraid_types::{
    Disk,
    DiskPartition, 
    PartitionMap, 
    PartitionSlices, 
    RaidMap,
    HyraidArray
};

use hyraid_lvm2::{
    lvm_lv_create,
    lvm_pv_create,
    lvm_vg_create,
    lvm_pv_resize,
    lvm_vg_extend
};

use hyraid_mdadm::{
    create_raid_array, 
    fail_from_raid_array, 
    remove_from_raid_array,
    add_to_raid_array
};

use hyraid_utils::{
    error_exit,
    unwrap_or_exit,
    unwrap_or_exit_verbose
};

use hyraid_gpt::{
    get_path_of_partition,
    clear_partitions,
    get_sector_size,
    ensure_gpt,
    get_free_space,
    validate_partition
};

use gpt;

use rand::Rng;

static HYRAID_JSON_PATH: &'static str = "/etc/hyraid.json";

fn random_string(length: usize) -> String {
    rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Generates slices from disks.
fn gen_slices(disks: &[&str]) -> PartitionSlices {
    let mut sizes = PartitionSlices::new();
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

/// Re-compute slices to account for larger disks being added
fn recompute_slices(disks: &[&str], slices: &PartitionSlices) -> PartitionSlices {
    let mut sizes = PartitionSlices::new();
    for disk in disks {
        sizes.push(get_free_space(disk));
    }

    sizes.sort_unstable();

    let mut slices: PartitionSlices = slices.clone();

    // skip first element (smallest disk size)
    for size in sizes[1..].iter() { 
        let slice = *size-slices.iter().sum::<usize>(); // Should probably rewrite this line.
        if slice != 0 {
            slices.push(slice)
        };
    }
    
    slices
}

/// Finds range from a vector whose sum is x
fn find_range_sum(vector: Vec<usize>,sum: usize) -> Vec<usize> {
    let mut x = 0;

    while vector[0..x].iter().sum::<usize>() != sum {
        x += 1;
    }

    vector[0..x].to_vec()
}

/// Lay-out partition map
fn make_partition_map(disks: &[&str], slices: &PartitionSlices) -> PartitionMap {
    let mut result = PartitionMap::new();

    for disk in disks {
        let size = get_free_space(disk);
        let part = find_range_sum(slices.clone(),size).iter().map(
            |x| {
                DiskPartition {
                    size: *x,
                    path: None
                }
            }
        ).collect();

        result.insert(disk.to_string(),part);
    }

    result
}

/// Creates partitions from partition map and returns same `PartitionMap`, 
/// this time with path of the partition included.
fn create_partition_map(part_map: PartitionMap) -> PartitionMap {
    let mut map = PartitionMap::new();
    for (disk,parts) in part_map {
        let sector_size = get_sector_size(&disk);
        let diskpath = std::path::Path::new(&disk);
        let mut gptdisk = unwrap_or_exit!(
            gpt::GptConfig::new()
                .writable(true)
                .open(diskpath),
            "Failed to open disk."
        );
        for part in parts {
            gptdisk.add_partition(
                "hyraid_partition",
                part.size.try_into().unwrap(),
                gpt::partition_types::LINUX_FS,
                0,
                None
            ).unwrap();
        }
        gptdisk.write().unwrap();
        let gptdisk = unwrap_or_exit!(
            gpt::GptConfig::new()
                .writable(true)
                .open(diskpath),
            "Failed to open disk."
        );
        let mut partitions: Vec<DiskPartition> = vec![];
        for (_,partition) in gptdisk.partitions() {
            let part_path = get_path_of_partition(&partition.clone());
            partitions.push(
                DiskPartition { 
                    path: Some(part_path), 
                    size: (partition.sectors_len().unwrap() * TryInto::<u64>::try_into(sector_size).unwrap())
                        .try_into()
                        .unwrap()
                }
            );
            validate_partition(partition.clone());
        }
        partitions.sort_by_key(|k| k.size);
        map.insert(disk,partitions);
    }

    map
}

/// Create initial RAID arrays
fn init_raid_map(part_map: PartitionMap) -> RaidMap {
    let mut raid_map = RaidMap::new();
    
    let mut part_map: Vec<(String,Vec<DiskPartition>)> = part_map.into_iter().collect();
    part_map.sort_unstable_by_key(|(_,parts)| parts.len());
    part_map.reverse();

    let mut groups: HashMap<usize,Vec<DiskPartition>> = HashMap::new();
    
    for i in 0..part_map[0].1.len() {
        groups.insert(i,vec![]);
    }

    for (group,partitions) in &mut groups {
        for (_,parts) in &part_map {
            if let Some(x) = parts.get(*group) {
                partitions.push(x.clone());
            }
        }
    }

    for group in groups.values() {
        let slice: Vec<DiskPartition> = group
            .iter()
            .map(
                |s| {
                    s.clone()
                }
            )
            .collect();
        let devname = &format!("/dev/md/hyraid_md_{}",random_string(10))[..];

        if slice.len() != 1 {
            raid_map.insert(devname.to_string(),slice);
        }
    }

    raid_map
}

/// Determine RAID level automatically
fn find_raid_level(partitions: usize,intended_raid_level: usize) -> usize {
    match intended_raid_level {
        0 => 0,
        1 => 1,
        5 => {
            if partitions < 3 {
                1 // RAID1
            } else {
                5 // RAID5
            }
        },
        6 => {
            if partitions < 3 {
                1 // RAID1
            } else {
                6 // RAID6
            }
        },

        _ => {
            error_exit!("Incorrect RAID level. Only RAID0,RAID1,RAID5 and RAID6 is supported.");
        }
    }
}

fn into_paths_slice(partitions: Vec<DiskPartition>) -> Vec<String> {
    let slice: Vec<String> = partitions
        .iter()
        .map(|s| s.path.clone().unwrap())
        .collect();

    slice.to_vec()
}

fn create_init_raid_map(raid_map: RaidMap,raid_level: usize) {
    for (raid_dev,partitions) in raid_map {
        let level = find_raid_level(partitions.len(),raid_level);

        let slice: Vec<String> = into_paths_slice(partitions);
        let slice: Vec<&str> = slice.iter().map(
            |s| s.as_str()
        ).collect();
        
        unwrap_or_exit_verbose!(
            create_raid_array(&raid_dev,&slice,level),
            "Error occurred while creating MD array"
        );
    }
}

/// Create LVM logical volume with all of the raid arrays.
/// basically combine the raid arrays into one.
fn create_lvm(raid_map: &RaidMap) -> String {
    let raid_arrays: &Vec<&str> = &raid_map.keys()
        .into_iter()
        .map(|s| s.as_str())
        .collect();
    let lv_name = format!("hyraid_vg_{}",random_string(16));
    unwrap_or_exit_verbose!(
        lvm_pv_create(raid_arrays),
        "Error occured while setting up LVM"
    );
    unwrap_or_exit_verbose!(
        lvm_vg_create(&lv_name[..],raid_arrays),
        "Error occured while setting up LVM"
    );
    unwrap_or_exit_verbose!(
        lvm_lv_create(&lv_name[..],raid_arrays,hyraid_lvm2::SizeFormat::EXTENTS,"100%FREE"),
        "Error occured while setting up LVM"
    );
    
    "/dev/".to_string() + &lv_name + &"/lvol0"
}

pub fn create_hyraid_array(name: String,disks: &[&str], raid_level: usize) -> String {
    for disk in disks {
        ensure_gpt(disk);
        clear_partitions(disk);
    }

    let slices = gen_slices(disks);
    
    let part_map = make_partition_map(disks,&slices);
    let part_map = create_partition_map(part_map);

    let raid_map = init_raid_map(part_map);
    create_init_raid_map(raid_map.clone(),raid_level.clone());
    // Combine disks
    let lvm_lv = create_lvm(&raid_map);

    if Path::new(&lvm_lv).exists() {
        let entry = HyraidArray {
            name: name,
            lvm_lv_path: lvm_lv.clone(),
            raid_level, 
            disks: disks
                .iter()
                .map(|&s| {
                    let diskpath = std::path::Path::new(&s);
                    let gptdisk = unwrap_or_exit!(
                        gpt::GptConfig::new()
                            .open(diskpath),
                        "Failed to open disk."
                    );
                    hyraid_types::Disk::from(gptdisk,get_sector_size(
                        diskpath
                            .as_os_str()
                            .to_str()
                            .unwrap()
                    ))
                }).collect(),
            raid_map: raid_map,
            slices,
        };
        hyraid_json::write_array(HYRAID_JSON_PATH,entry);
    }

    lvm_lv
}

pub fn fail_from_hyraid_array(name: String, disks: &[&str]) {
    for disk in disks {
        let mut partitions: Vec<DiskPartition> = vec![];
        let gptdisk = unwrap_or_exit!(
            gpt::GptConfig::new()
                .open(disk),
            "Failed to open disk."
        );
        for partition in gptdisk.partitions().values() {
            partitions.push(DiskPartition::from(partition));
        }
        match hyraid_json::read_arrays(HYRAID_JSON_PATH).iter().find(|x| x.name == name) {
            Some(entry) => {
                for part in partitions.clone() {
                    let raid_array = entry.raid_map
                        .iter()
                        .find(|x| x.1.contains(&part));
                    if let Some(array) = raid_array {
                        for partition in &partitions {
                            if array.1.contains(&partition) {
                                unwrap_or_exit_verbose!(
                                    fail_from_raid_array(array.0,&[&partition.path.clone().unwrap().as_str()]),
                                    "Failed to mark drive as faulty. mdadm output:"
                                );
                                println!("Marked disk(s) as faulty on array.");
                            }
                        }
                    }
                } 
            },
            None => {
                error_exit!("Error: No such HyRAID array.");
            }
        }
    }
}

pub fn add_disk_to_hyraid_array(name: String, disks: &[&str]) {
    match hyraid_json::read_arrays(HYRAID_JSON_PATH).iter().find(|x| x.name == name) {
        Some(entry) => {
            // Get all the stuff we need from the 
            let mut raid_map_entry = entry.raid_map.clone();
            let slices = &entry.slices;

            // Re-compute the slices to account for larger disks being added
            // since a larger disk means the current slices won't be enough
            let slices = &recompute_slices(disks,slices);

            let part_map = make_partition_map(disks,slices);
            let part_map = create_partition_map(part_map);
            let raid_map = init_raid_map(part_map);
            
            for (array_name,partitions) in &raid_map.clone() {
                let raid_array = raid_map_entry.iter().find(
                    |x| &x.1[0].size == &partitions[0].size
                );
                
                if let Some((array,array_partitions)) = raid_array {
                    let slice = into_paths_slice(partitions.to_vec());
                    let slice: Vec<&str> = slice.iter().map(
                        |s| s.as_str()
                    ).collect();
                    unwrap_or_exit_verbose!(
                        add_to_raid_array(&array,&slice),
                        "Failed to add disk to array. mdadm output:"
                    );
                    unwrap_or_exit_verbose!(
                        lvm_pv_resize(&[&array]),
                        "Failed to add disk to array. LVM (lvresize) output:"
                    );
                    raid_map_entry.insert(array.to_string(),array_partitions.to_vec());
                } else {
                    let slice = into_paths_slice(partitions.to_vec());
                    let slice: Vec<&str> = slice.iter().map(
                        |s| s.as_str()
                    ).collect();
                    let level = find_raid_level(slice.len(),entry.raid_level);
                    unwrap_or_exit_verbose!(
                        create_raid_array(&array_name,&slice,level),
                        "Failed to add disk to array. mdadm output:"
                    );
                    unwrap_or_exit_verbose!(
                        lvm_pv_create(&[&array_name]),
                        "Failed to add disk to array. LVM (lvresize) output:"
                    );
                    unwrap_or_exit_verbose!(
                        lvm_vg_extend(entry.lvm_lv_path.trim_end_matches("/lvol0"),&[&array_name]),
                        "Failed to add disk to array. LVM (lvresize) output:"
                    );
                    raid_map_entry.insert(array_name.to_string(),partitions.to_vec());
                }

                let mut disks_entry = entry.disks.clone();
                for (_,disk) in &raid_map {
                    disks_entry.push(Disk{
                        partitions: disk.to_vec(),
                    })
                }

                hyraid_json::modify(HYRAID_JSON_PATH,entry.name.clone(),HyraidArray {
                    name: entry.name.clone(),
                    lvm_lv_path: entry.lvm_lv_path.clone(),
                    disks: disks_entry.to_vec(),
                    slices: slices.to_vec(),
                    raid_level: entry.raid_level,
                    raid_map: raid_map.clone()
                });
            }
        },
        None => {
            error_exit!("Error: No such HyRAID array.");
        }
    }
}

pub fn remove_disk_from_array(name: String, disks: &[&str]) {
    for disk in disks {
        let mut partitions: Vec<DiskPartition> = vec![];
        let gptdisk = unwrap_or_exit!(
            gpt::GptConfig::new()
                .open(disk),
            "Failed to open disk."
        );
        for partition in gptdisk.partitions().values() {
            partitions.push(DiskPartition::from(partition));
        }
        match hyraid_json::read_arrays(HYRAID_JSON_PATH).iter().find(|x| x.name == name) {
            Some(entry) => {
                for part in partitions.clone() {
                    let raid_array = entry.raid_map
                        .iter()
                        .find(|x| x.1.contains(&part));
                    if let Some(array) = raid_array {
                        for partition in &partitions {
                            if array.1.contains(&partition) {
                                unwrap_or_exit_verbose!(
                                    remove_from_raid_array(array.0,&[&partition.path.clone().unwrap().as_str()]),
                                    "Failed to remove disk(s). mdadm output:"
                                );
                                println!("Removed disk(s) from array.");
                            }
                        }
                    }
                } 
            },
            None => {
                error_exit!("Error: No such HyRAID array.");
            }
        }
    }
}