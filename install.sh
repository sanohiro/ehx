#!/bin/sh
# ehx apt repository installer
# Usage: curl -fsSL https://sanohiro.github.io/ehx/install.sh | sudo sh

set -e

# Add GPG key
curl -fsSL https://sanohiro.github.io/ehx/ehx.gpg | gpg --dearmor -o /usr/share/keyrings/ehx.gpg

# Add repository
echo "deb [signed-by=/usr/share/keyrings/ehx.gpg] https://sanohiro.github.io/ehx stable main" > /etc/apt/sources.list.d/ehx.list

# Update package list
apt update

echo "Done! Run 'apt install ehx' to install."
