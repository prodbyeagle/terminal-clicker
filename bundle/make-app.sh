#!/bin/sh
# Baut ein startbares "Terminal Clicker.app" unter target/release/.
set -e
cd "$(dirname "$0")/.."
cargo build --release
APP="target/release/Terminal Clicker.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp bundle/Info.plist "$APP/Contents/Info.plist"
cp target/release/terminal-clicker "$APP/Contents/MacOS/terminal-clicker"
echo "fertig: $APP"
