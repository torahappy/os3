#!/bin/bash

yarn electron-forge make -a arm64 -p linux
yarn electron-forge make -a x64 -p linux
yarn electron-forge make -a ia32 -p linux
yarn electron-forge make -a arm64 -p darwin
yarn electron-forge make -a x64 -p darwin
yarn electron-forge make -a arm64 -p windows
yarn electron-forge make -a x64 -p windows
yarn electron-forge make -a ia32 -p windows
