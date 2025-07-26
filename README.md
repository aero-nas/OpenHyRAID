# OpenHyRAID

STILL working in progress!

<p align="center">
    <img src="img/hyraid.svg"
      height="100"
      style="padding:16px;"  
    >
</p>

Open-source, high-level mdadm wrapper to implement Synology SHR written in Rust.

# What is Synology SHR

SHR is a proprietary raid level that allows adding drives that are larger without wasting space. If you create a RAID array with 3 disks, 2 10TB and 1 15TB, you will lose 5TB of space. HyRAID and SHR, allow you to add larger disks by automatically partitioning the disks so you don't lose space.

# SHR vs HyRAID

HyRAID supports RAID0,RAID1,RAID5, and RAID6. While SHR only supports RAID5 (SHR-1) and RAID6 (SHR-2)

HyRAID is free and licensed under GPLv2. While SHR is proprietary.

HyRAID has priority aarch64 (architecture of Raspberry Pi 5/CM5) support.

made with ❤️ by lizard >w<

# Roadmap
Necessary for 1.0
  - [ ] Creating arrays
  - [ ] Modifying arrays (e.g. Replacing a drive)