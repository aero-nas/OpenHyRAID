/*!
    Type aliases and structs used for HyRAID

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

use gpt::partition::Partition;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use hyraid_gpt;

/**
Raid Map

a map containing raid arrays and what partitions are on them.
*/
pub type RaidMap = HashMap<String,Vec<DiskPartition>>;

/**
Partition map

a map containing disks' path, e.g. /dev/sda...

and the partitions inside them.
*/
pub type PartitionMap = HashMap<String,Vec<DiskPartition>>;

/**
Partition slices

A list of partition sizes in sectors.

Used to determine how many partitions are created, and of what size.
*/
pub type PartitionSlices = Vec<usize>;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct DiskPartition {
    pub path: Option<String>,
    pub size: usize
}

impl DiskPartition {
    pub fn from(partition: &Partition) -> Self {
        Self {
            path: Some(hyraid_gpt::get_path_of_partition(&partition)),
            size: (
                partition
                    .sectors_len()
                    .unwrap() 
                    * 
                TryInto::<u64>::try_into(
                    hyraid_gpt::get_sector_size(
                        &hyraid_gpt::get_path_of_partition(&partition))
                    ).unwrap()
            ).try_into().unwrap()
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Disk {
    pub partitions: Vec<DiskPartition>
}

impl Disk {
    pub fn from(disk: gpt::GptDisk<std::fs::File>,sector_size: usize) -> Self {
        let mut parts: Vec<DiskPartition> = vec![];
        for partition in disk.partitions().values() {
            parts.push(
                DiskPartition { 
                    path: Some(hyraid_gpt::get_path_of_partition(&partition)),
                    size: (
                        partition.sectors_len().unwrap() 
                        * 
                        TryInto::<u64>::try_into(sector_size).unwrap()
                    ).try_into().unwrap()
                }
            )
        };
        Self { partitions: parts }
    }
}

/// Struct representing a HyRAID array.
/// 
/// Can be (de)serialized with serde
#[derive(Serialize, Deserialize, Clone)]
pub struct HyraidArray {
    pub name: String,
    pub lvm_lv_path: String,
    pub raid_level: usize,
    pub disks: Vec<Disk>,
    pub raid_map: RaidMap,
    pub slices: PartitionSlices
}




