#!/bin/bash

xcode-select --install

/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

echo >> /Users/admin/.zprofile
echo 'eval "$(/opt/homebrew/bin/brew shellenv zsh)"' >> /Users/admin/.zprofile
eval "$(/opt/homebrew/bin/brew shellenv zsh)"

brew install cloudflared nvim python wget mpv git-lfs cmake
