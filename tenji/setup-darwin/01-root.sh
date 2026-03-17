#!/bin/bash

# run as administrator account but not via sudo

xcode-select --install

/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

echo >> $HOME/.zprofile
echo 'eval "$(/opt/homebrew/bin/brew shellenv zsh)"' >> $HOME/.zprofile
eval "$(/opt/homebrew/bin/brew shellenv zsh)"

brew install cloudflared nvim python wget mpv git-lfs cmake
