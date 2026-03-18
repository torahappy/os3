#!/bin/bash

cd $HOME

echo >> ~/.zprofile
echo 'autoload -Uz compinit && compinit -u' >> ~/.zprofile
echo 'eval "$(/opt/homebrew/bin/brew shellenv zsh)"' >> ~/.zprofile
echo >> ~/.zshrc
echo 'alias gP="git push origin ; git push origin2"' >> ~/.zshrc

. $HOME/.zprofile

git lfs install

curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.4/install.sh | bash

echo 'export NVM_DIR="$HOME/.nvm"; [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"' >> $HOME/.zprofile

. $HOME/.zprofile

nvm install --lts

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

echo '. "$HOME/.cargo/env"' >> $HOME/.zprofile

. $HOME/.zprofile
 
git clone https://gitlab.torahappy.org/tora/os3 

cd os3

git remote add origin2 https://github.com/torahappy/os3

git checkout tenji2603

cd os3bevy; rustup target add wasm32-unknown-unknown; cargo install -f wasm-bindgen-cli --version 0.2.108; cargo install wasm-opt; cd ..

./build_scripts/wasm-once.sh os3yew shimbun

cd python-tts
./setup.sh
cd ..

cd tenji/electron
npm install -g yarn
yarn
cd ../../

./external-sources/setup-tts.sh

cd tenji/setup-darwin

mkdir $HOME/Library/LaunchAgents
cp com.torahappy.shimbunstartup.plist $HOME/Library/LaunchAgents

cd ../../
