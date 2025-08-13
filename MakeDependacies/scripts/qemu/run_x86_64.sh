#!/bin/bash
set -e

ARCH=x86_64
OUTPUT_DIR=output-${ARCH}

qemu-system-x86_64 \
  -m 2048 \
  -cpu host \
  -enable-kvm \
  -drive if=pflash,format=raw,readonly,file=firmware/OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=firmware/OVMF_VARS.fd \
  -drive file=${OUTPUT_DIR}/images/nexisos.iso,media=cdrom,readonly=on \
  -netdev user,id=net0 \
  -device e1000,netdev=net0 \
  -device virtio-serial-pci \
  -boot d \
  -display default
