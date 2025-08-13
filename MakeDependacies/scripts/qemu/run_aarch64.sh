#!/bin/bash
set -e

ARCH=aarch64
OUTPUT_DIR=output-${ARCH}

qemu-system-aarch64 \
  -m 2048 \
  -cpu cortex-a57 \
  -machine virt,secure=off \
  -bios firmware/QEMU_EFI.fd \
  -drive if=none,file=${OUTPUT_DIR}/images/nexisos.iso,format=raw,id=cdrom \
  -device virtio-blk-device,drive=cdrom \
  -netdev user,id=net0 \
  -device virtio-net-device,netdev=net0 \
  -boot d \
  -display default
