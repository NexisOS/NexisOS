{ pkgs ? import <nixpkgs> {} }:

(pkgs.buildFHSEnv {
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
    gnutar
    zlib
    glibc
    diffutils
    gettext
    xz
    gzip
    bzip2
    lzop
    lz4
    zstd
    pkg-config
    autoconf
    automake
    libtool
    texinfo
    openssl.dev
    curl
    subversion
  ];

  runScript = "bash";

}).env
