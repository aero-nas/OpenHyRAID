# Using docker for CI/CD

> [!WARNING] CI/CD requires binfmt support to cross-compile for aarch64

> [!CAUTION] While running any of the containers, no loopdevices or HyRAID arrays must be active. Otherwise, it will either, fail, or erase any active HyRAID arrays.

 - scripts:
   ---
   main.sh -> Runs on every docker container after being prepared
 - Dockerfiles:
   ---
   There is a seperate dockerfile for each platform. It prepares the environment for the scripts and installs dependencies needed for that platform.
