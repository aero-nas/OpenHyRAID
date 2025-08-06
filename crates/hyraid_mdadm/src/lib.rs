/*!
    mdadm bindings
    
    Similar behaviour to libblockdev

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

use hyraid_utils::run_cmd;
use std::process::Command;
use regex::Regex;

/// Macro to check if device name is correct.
macro_rules! check_dev {
    ($dev:expr) => {
        if !$dev.starts_with("/dev/") {
            return Err(format!("Incorrect device path: {}",$dev))
        }
    }
}

/// Macro to check if multiple device names are correct.
macro_rules! check_devs {
    ($devs:expr) => {
        for dev in $devs.iter() {
            check_dev!(dev);
        }
    }
}


/// Represents a software RAID device (array)
#[derive(Debug)]
pub struct RaidDev {
    /// Devices active in the RAID array
    pub raid_devices: usize,

    /// Array creation time
    pub created_at: String,

    /// Array update time
    pub updated_at: String,

    /// RAID level (e.g. raid5)
    pub level: String,

    /// Total usable size of the RAID array
    pub full_size: usize,

    /// Used size of the RAID array
    pub used_size: usize,

    /// Total amount of devices in the raid array including ones that are not active.
    pub all_devices: usize,

    /// States of the RAID array
    pub state: Vec<String>,

    /// Amount of failed devices
    pub failed_devices: usize,

    /// Amount of active devices
    pub active_devices: usize,

    /// Amount of working devices
    pub working_devices: usize,

    /// Amount of spare devices
    pub spare_devices: usize,

    /// Array layout
    pub layout: String,

    /// Chunk size
    pub chunk_size: String,

    /// RAID device UUID
    pub uuid: String,

    /// Name of the RAID array
    pub name: String,

    /// Events
    pub events: usize,
}

/// Create new (regular) RAID array, return error as a string if failed.
pub fn create_raid_array(
    raid_dev: &str,
    partitions: &[&str],
    raid_level:usize
) -> Result<(),String> {
        let mut cmd = Command::new("mdadm");
        if !([0,1,5,6].contains(&raid_level)) {
            return Err(format!("Incorrect raid level:{}",raid_level));
        }
        
        if raid_dev.len() > 32 {
            return Err(String::from("md device name too long (32 max)"));
        }
        
        check_devs!(partitions);
        check_dev!(raid_dev);
        
        cmd.arg("--create");
        
        let strip = raid_dev
            .trim_start_matches("/dev/")
            .trim_start_matches("md")
            .trim_start_matches("/");
    
        if strip.parse::<usize>().is_ok() {
            cmd.arg(format!("/dev/md{}",strip));
        } else {
            cmd.arg(format!("/dev/md/{}",strip));
        }
        
        cmd.args(["--metadata","1.2"]);
        cmd.arg(format!("--level={}",&raid_level.to_string()));
        cmd.args(["--raid-devices",&partitions.len().to_string()]);
        cmd.arg("--bitmap=internal");
        cmd.args(partitions);

        run_cmd!(cmd)
    }


/// Mark partitions as faulty, return error as a string if failed.
pub fn fail_from_raid_array(
    raid_dev: &str,
    partitions: &[&str],
) -> Result<(),String> {
    let mut cmd = Command::new("mdadm");

    check_dev!(raid_dev);
    check_devs!(partitions);

    cmd.arg(raid_dev);
    cmd.arg("--fail");
    cmd.args(partitions);
    println!("{:?}",cmd.get_args());
    run_cmd!(cmd)
}

/// Remove partition from array, return error as a string if failed.
pub fn remove_from_raid_array(
    raid_dev: &str,
    partitions: &[&str],
) -> Result<(),String> {
    let mut cmd = Command::new("mdadm");
    
    check_dev!(raid_dev);
    check_devs!(partitions);
    
    cmd.arg(raid_dev);
    cmd.arg("--remove");
    cmd.args(partitions);

    run_cmd!(cmd)
}

pub fn grow_raid_array(
    raid_dev: &str,
    raid_level:usize
) -> Result<(),String> {
    let mut cmd = Command::new("mdadm");
    
    check_dev!(raid_dev);
    
    cmd.arg("--grow");
    cmd.arg(raid_dev);
    cmd.arg(format!("--level={}",&raid_level.to_string()));

    run_cmd!(cmd)
}

/// Add partitions to existing array, return error as a string if failed.
pub fn add_to_raid_array(
    raid_dev: &str,
    partitions: &[&str],
) -> Result<(),String> {
    let mut cmd = Command::new("mdadm");
    
    check_dev!(raid_dev);
    check_devs!(partitions);
    
    cmd.arg(raid_dev);
    cmd.arg("--add");
    cmd.args(partitions);

    run_cmd!(cmd)
}

/// Returns true if the partition is in a raid array
pub fn is_part_in_raid_array(
    dev: &str
) -> Result<bool,String> {
    let cmd = Command::new("mdadm")
        .arg("--detail")
        .arg(dev)
        .output()
        .unwrap();

    if cmd.stderr != b"" {
        return Err(String::from_utf8(cmd.stderr).unwrap())
    }

    let stdout = String::from_utf8(cmd.stdout).unwrap();

    // if the partition is in an array, 
    // mdadm will output a bunch of stuff
    // which will contain MD_UUID=whatever
    // kind of a hack but should work perfectly
    Ok(stdout.contains("MD_UUID")) 
}

/// Get details of RAID array, return error as a string if failed.
pub fn get_detail(
    raid_dev: &str
) -> Result<RaidDev, String> {
    let cmd = Command::new("mdadm")
        .arg("--detail")
        .arg(raid_dev)
        .output()
        .unwrap();

    if cmd.stderr != b"" {
        return Err(String::from_utf8(cmd.stderr).unwrap())
    }

    let stdout = String::from_utf8(cmd.stdout).unwrap();

    let regex_list = [
        r"Creation Time : (?P<created_at>.+)",
        r"Raid Level : (?P<level>.+)",

        // holy clusterfuck
        r"Array Size : (?P<full_size>\d+)[\(\)\d\s\.a-zA-Z]*",
        r"Used Dev Size : (?P<used_size>\d+)[\(\)\d\s\.a-zA-Z]*",

        r"Raid Devices : (?P<raid_devices>\d+)",
        r"Total Devices : (?P<all_devices>\d+)[\sA-Za-z:]*",
        r"Update Time : (?P<updated_at>.+)",
        r"State : (?P<state>.+)",
        r"Active Devices : (?P<active_devices>\d+)",
        r"Working Devices : (?P<working_devices>\d+)",
        r"Failed Devices : (?P<failed_devices>\d+)",
        r"Spare Devices : (?P<spare_devices>\d+)",
        r"Layout : (?P<layout>.+)",
        r"Chunk Size : (?P<chunk_size>.+)[\s:a-zA-Z]*",
        r"Name : .*:(?P<name>\S+)[\sa-z\(\)]*",
        r"UUID : (?P<uuid>.+)",
        r"Events : (?P<events>.+)",
    ];

    let regex = Regex::new(&regex_list.join(r"[\s.]*")[..])
        .unwrap()
        .captures(&stdout)
        .unwrap();
    
    let updated_at = regex["updated_at"].to_string();
    let level = regex["level"].to_string();
    let active_devices = regex["active_devices"].to_string();
    let spare_devices = regex["spare_devices"].to_string();
    let failed_devices = regex["failed_devices"].to_string();
    let working_devices = regex["working_devices"].to_string();
    let uuid = regex["uuid"].to_string();
    let full_size = regex["full_size"].to_string();
    let used_size = regex["used_size"].to_string();
    let all_devices = regex["all_devices"].to_string();
    let state = regex["state"].replace(" ","").split(",")
        .collect::<Vec<_>>()
        .iter().map(|&x| x.into())
        .collect();
    let name = regex["name"].to_string();
    let events = regex["events"].to_string();
    let layout = regex["layout"].to_string();
    let chunk_size = regex["chunk_size"].to_string();

    Ok(
        RaidDev {
            raid_devices: 
                regex["raid_devices"]
                    .to_string()
                    .parse::<usize>()
                    .unwrap(),
            working_devices: 
                working_devices
                    .parse::<usize>()
                    .unwrap(),
            active_devices: 
                active_devices
                    .parse::<usize>()
                    .unwrap(),
            all_devices:
                all_devices
                    .parse::<usize>()
                    .unwrap(),
            failed_devices: 
                failed_devices
                    .parse::<usize>()
                    .unwrap(),
            spare_devices: 
                spare_devices
                    .parse::<usize>()
                    .unwrap(),                
            full_size: 
                full_size
                    .parse::<usize>()
                    .unwrap(),
            used_size: 
                used_size
                    .parse::<usize>()
                    .unwrap(),
            events:
                events
                    .parse::<usize>()
                    .unwrap(),

            level: level,
            layout: layout,
            created_at: regex["created_at"].to_string(),
            updated_at: updated_at,
            uuid: uuid,
            name: name,
            state: state,
            chunk_size: chunk_size,
        }
    )
}