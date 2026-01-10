#!/bin/sh
# hx apt repository installer
# Usage: curl -fsSL https://sanohiro.github.io/hx/install.sh | sudo sh

set -e

# Add GPG key
curl -fsSL https://sanohiro.github.io/hx/hx.gpg | gpg --dearmor -o /usr/share/keyrings/hx.gpg

# Add repository
echo "deb [signed-by=/usr/share/keyrings/hx.gpg] https://sanohiro.github.io/hx stable main" > /etc/apt/sources.list.d/hx.list

# Update package list
apt update

echo "Done! Run 'apt install hx' to install."
