/*
    Open source implementation of Synology SHR.

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

use hyraid_mapper;
use hyraid_utils::is_root;
use std::{
    io::{self, Write}, process
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version = "v0.1", about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Create {
        /// Name of the HyRAID array
        #[arg(long = "array-name", value_name = "Array name")]
        name: String,
        
        /// Intended RAID level
        #[arg(long, value_name = "RAID level")]
        raid_level: usize,

        /// Disks to use
        disks: Vec<String>
    },
    Fail {
        /// Name of the HyRAID array
        #[arg(long = "array-name", value_name = "Array name")]
        name: String,

        disks: Vec<String>
    },
    Remove {
        /// Name of the HyRAID array
        #[arg(long = "array-name", value_name = "Array name")]
        name: String,

        /// Disks to use
        disks: Vec<String>
    },
    Add {
        /// Name of the HyRAID array
        #[arg(long = "array-name", value_name = "Array name")]
        name: String,

        /// Disks to use
        disks: Vec<String>
    },
}

fn cli_input(prompt: &str) -> String {
    print!("{}",prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    let input = input.trim();

    input.to_string()
}

fn main() {
    if !is_root() {
        println!("HyRAID must be run as root. Quitting.");
        process::exit(1);
    }
    let cli = Cli::parse();
    println!("THIS PROGRAM IS IN W.I.P. RUNNING IT MAY RESULT IN UNDEFINED BEHAVIOUR!!!");
    match &cli.command {
        Commands::Create { disks, raid_level, name } => {
            let mut answer = " ".to_string();
            if !(answer == "" || answer == "y" || answer == "n"){
                while !(answer == "" || answer == "y" || answer == "n") {
                    answer = cli_input("All data on the disks will be lost. Are you sure? [y/N]: ").to_lowercase();
                }
            }
            
            if answer == "n" || answer == "" {
                println!("Cancelled.");
                process::exit(0);
            }
            
            let slice = &disks
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>();

            let logical_volume = hyraid_mapper::create_hyraid_array(name.to_string(),slice,*raid_level);
            println!("Created logical volume: {}",logical_volume);
        }
        Commands::Fail { name, disks } => {
            let slice = &disks
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>();

            hyraid_mapper::fail_from_hyraid_array(name.to_string(),slice);
        },
        Commands::Add { name, disks } => {
            let slice = &disks
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>();

            hyraid_mapper::add_disk_to_hyraid_array(name.to_string(),slice);
        },
        Commands::Remove { name, disks } => {
            let slice = &disks
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>();

            hyraid_mapper::remove_disk_from_array(name.to_string(),slice);
        },
    }
}