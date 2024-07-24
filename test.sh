#!/bin/bash

# Check if the correct number of arguments are provided
if [ "$#" -ne 3 ]; then
  echo "Usage: $0 <repo_url> <branch> <commit_hash>"
  exit 1
fi

# Assign input arguments to variables
repo_url=$1
branch=$2
commit_hash=$3

# Clone the repository with the specified branch and commit hash
git clone -b "$branch" "$repo_url" "./$commit_hash" || echo "Exit Code: 1"; exit 1

# Change directory to the cloned repository
cd "./$commit_hash" || echo "Exit Code: 1"; exit 1

# Checkout the specific commit hash
git checkout "$commit_hash" || echo "Exit Code: 1"; exit 1

# Update submodules
git submodule update --init --recursive || echo "Exit Code: 1"; exit 1

# Build the project using cargo
cargo build --workspace --release || echo "Exit Code: 1"; exit 1

# Compile sylib.cc with GCC
cd project-eval/runtime || echo "Exit Code: 1"; exit 1|| exit
gcc -march=rv64gc -mabi=lp64d -xc++ -O2 -c -o sylib.o sylib.cc || echo "Exit Code: 1"; exit 1

# Create the static library
ar rcs libsysy.a sylib.o || echo "Exit Code: 1"; exit 1

# Run functional tests
cd ../ || echo "Exit Code: 1"; exit 1
python3 test.py -t ./testcases/functional -b -c gcc -r cmmc --on_riscv || echo "Exit Code: 1"; exit 1

# Run performance tests
python3 test.py -t ./testcases/performance -b -c gcc -r cmmc --on_riscv || echo "Exit Code: 1"; exit 1

# Clean up by removing the cloned repository directory
cd ../../ || echo "Exit Code: 1"; exit 1
rm -rf "./$commit_hash" || echo "Exit Code: 1"; exit 1
