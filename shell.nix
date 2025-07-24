{ pkgs ? import <nixpkgs> {} }:

pkgs.buildFHSEnv {
  name = "buildroot-env";
  multiPkgs = pkgs: with pkgs; [
    bash
    coreutils
    gcc
    gnumake
    binutils
    bison
    flex
    bc
    perl
    unzip
    cpio
    rsync
    which
    file
    python3
    git
    wget
    patch
    ncurses.dev
    findutils
    util-linux
    gawk
    tar
    zlib
    glibc
  ];

  runScript = "bash";

}.env
