#!/usr/bin/env sh
# We need this because I use Arch linux on my dev machine which uses a really recent version of
# glibc, meaning that ubuntu with it's older version can't run the game. Using Musl might be an
# option too but when I briefly tried it I couldn't get window creation to work because of how X11
# was linked.
docker run --user "$(id -u)":"$(id -g)" -v "$PWD":/cheese -w /cheese rust:latest cargo build --release
# Verify that we're not linking against glibc 2.32:
echo 'Glibc 2.2x and 2.3x links:'
readelf -Ws target/release/cheese | rg 'GLIBC_2.2[^\. ]'
readelf -Ws target/release/cheese | rg 'GLIBC_2.3[^\. ]'
mkdir -p linux_build
rm -rf linux_build/*
cp controls.md linux_build
cp target/release/cheese linux_build
