/*!
    lvm2 bindings
    
    Similar behaviour to libblockdev for C

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

use std::{process::Command};
use hyraid_utils::run_cmd;

pub enum SizeFormat {
    EXTENTS,
    SIZE
}

/// Initialize LVM Physical Volume
pub fn lvm_pv_create(partitions: &[&str]) -> Result<(),String> {
    let mut output = Command::new("pvcreate");    
    output.args(partitions);
    println!("{:?}",output.get_args());
    run_cmd!(output)
}

/// Create LVM Volume Group
pub fn lvm_vg_create(group_name: &str, partitions: &[&str]) -> Result<(),String> {
    let mut output = Command::new("vgcreate");
    output.arg(group_name);
    output.args(partitions);
    println!("{:?}",output.get_args());
    run_cmd!(output)
}

/// Create LVM Logical Volume
pub fn lvm_lv_create(group_name: &str, partitions: &[&str], size_type: SizeFormat, size: &str) -> Result<(),String> {
    let mut output = Command::new("lvcreate");
    output.arg(group_name);
    output.args(partitions);
    match size_type {
        SizeFormat::EXTENTS => output.arg("-l"),
        SizeFormat::SIZE => output.arg("-L")
    };
    output.arg(size);
    println!("{:?}",output.get_args());
    run_cmd!(output)
}

/// Resize Physical Volume
pub fn lvm_pv_resize(partitions: &[&str]) -> Result<(),String> {
    let mut output = Command::new("pvresize");
    output.args(partitions);
    run_cmd!(output)
}

/// Resize Logical Volume
pub fn lvm_lv_resize(partition: &str, resizefs: bool, size_type: SizeFormat, size: &str) -> Result<(),String> {
    let mut output = Command::new("lvresize");
    output.arg(partition);
    if resizefs {
        output.arg("--resizefs");
    };
    match size_type {
        SizeFormat::EXTENTS => output.arg("-l"),
        SizeFormat::SIZE => output.arg("-L")
    };
    output.arg(size);
    run_cmd!(output)
}

/// Add Physical Volume to Volume Group
pub fn lvm_vg_extend(group_name: &str, partitions: &[&str]) -> Result<(),String> {
    let mut output = Command::new("vgextend");
    output.arg(group_name);
    output.args(partitions);
    run_cmd!(output)
}