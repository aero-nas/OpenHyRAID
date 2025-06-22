/**
 * Parse CLI arguments
 * 
 * Copyright (C) 2025 LIZARD-OFFICIAL-77
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 */

pub use clap::Parser;

/// Default chunk size for mdadm.
const DEFAULT_CHUNKSIZE: &'static str = "512K";

#[derive(Parser,Debug)]
#[command(about, long_about = None,version)]
pub struct CLI {
    /// Create mode - create new HyRAID array
    #[arg(long,short = 'C')]
    pub create: bool,

    /// Extend mode - add disk to HyRAID array
    #[arg(long,short = 'E')]
    pub extend: bool,
    
    /// Format a disk for use in an HyRAID array
    #[arg(long,short = 'F')]
    pub format: bool,

    /// Reduce mode - remove disk from array
    #[arg(long,short = 'R')]
    pub reduce: bool,
    
    /// Chunk size for HyRAID array - pointless for RAID1
    #[arg(short,long,default_value_t=DEFAULT_CHUNKSIZE.to_string())]
    pub chunk: String,

    /// List of disks
    pub disks: Vec<String>,
}