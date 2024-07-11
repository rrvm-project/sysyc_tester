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
git clone -b "$branch" "$repo_url" "./$commit_hash"

# Change directory to the cloned repository
cd "./$commit_hash" || exit

# Checkout the specific commit hash
git checkout "$commit_hash"

# Update submodules
git submodule update --init --recursive

# Build the project using cargo
cargo build --workspace --release

# Compile sylib.cc with GCC
cd project-eval/runtime || exit
gcc -march=rv64gc -mabi=lp64d -xc++ -O2 -c -o sylib.o sylib.cc

# Create the static library
ar rcs libsysy.a sylib.o

# Run functional tests
cd ../ || exit
python3 test.py -t ./testcases/functional -b -c gcc --on_riscv

# Run performance tests
python3 test.py -t ./testcases/performance -b -c gcc --on_riscv

# Clean up by removing the cloned repository directory
cd ../ || exit
# rm -rf "./$commit_hash"
