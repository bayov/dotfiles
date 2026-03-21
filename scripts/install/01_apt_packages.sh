#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing APT Packages"
echo "================================================================================"
echo -e "\033[0m"

sudo apt-get update

# install general pre-requisites
sudo apt-get install -y build-essential
sudo apt-get install -y pkg-config
sudo apt-get install -y libssl-dev
sudo apt-get install -y unzip
sudo apt-get install -y git
sudo apt-get install -y curl
sudo apt-get install -y ca-certificates
sudo apt-get install -y python3
sudo apt-get install -y python3-venv

# useful basic tools
sudo apt-get install -y fd-find
sudo apt-get install -y ripgrep
sudo apt-get install -y btop
sudo apt-get install -y bat
