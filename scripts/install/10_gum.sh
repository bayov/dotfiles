#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Gum"
echo "================================================================================"
echo -e "\033[0m"

sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://repo.charm.sh/apt/gpg.key | sudo gpg --dearmor --yes -o /etc/apt/keyrings/charm.gpg || true
echo "deb [signed-by=/etc/apt/keyrings/charm.gpg] https://repo.charm.sh/apt/ * *" | sudo tee /etc/apt/sources.list.d/charm.list
sudo apt-get update
sudo apt-get install -y gum

# To update just run again
