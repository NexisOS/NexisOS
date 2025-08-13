#!/bin/bash
set -e

ARCH=riscv64
OUTPUT_DIR=output-${ARCH}

qemu-system-riscv64 \
  -m 2048 \
  -machine virt \
  -bios default \
  -kernel ${OUTPUT_DIR}/images/Image \
  -append "root=/dev/vda rw console=ttyS0" \
  -drive file=${OUTPUT_DIR}/images/rootfs.ext4,format=raw,id=hd0 \
  -device virtio-blk-device,drive=hd0 \
  -netdev user,id=net0 \
  -device virtio-net-device,netdev=net0 \
  -nographic \
  -display default
