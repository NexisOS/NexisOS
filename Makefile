.DEFAULT_GOAL := build

# === Variables ===

ARCH ?= x86_64
DEFCONFIG := NexisOS_$(ARCH)_defconfig
CONFIG_FILE := $(abspath depends/configs/$(DEFCONFIG))
KERNEL_CONFIG := $(abspath depends/kernel-configs/linux-$(ARCH).config)
BUILDROOT_VERSION := 2025.xx.xx
BUILDROOT_DIR := ../buildroot
OUTPUT_DIR := output-$(ARCH)

BACKUP_CONFIG := $(BUILDROOT_DIR)/package/Config.in.bak
BUILDROOT_CONFIG := $(BUILDROOT_DIR)/package/Config.in

# === QEMU flags per architecture ===

QEMU_x86_64 = qemu-system-x86_64 -m 2048 -cpu host -enable-kvm \
	-drive if=pflash,format=raw,readonly,file=firmware/OVMF_CODE.fd \
	-drive if=pflash,format=raw,file=firmware/OVMF_VARS.fd \
	-drive file=$(OUTPUT_DIR)/images/nexisos.iso,media=cdrom,readonly=on \
	-netdev user,id=net0 -device e1000,netdev=net0 \
	-device virtio-serial-pci \
	-serial stdio \
	-boot d

QEMU_aarch64 = qemu-system-aarch64 -m 2048 -cpu cortex-a57 -machine virt,secure=off \
	-bios firmware/QEMU_EFI.fd \
	-drive if=none,file=$(OUTPUT_DIR)/images/nexisos.iso,format=raw,id=cdrom \
	-device virtio-blk-device,drive=cdrom \
	-netdev user,id=net0 -device virtio-net-device,netdev=net0 \
	-serial stdio \
	-boot d

QEMU_riscv64 = qemu-system-riscv64 -m 2048 -machine virt -bios default \
	-kernel $(OUTPUT_DIR)/images/Image \
	-append "root=/dev/vda rw console=ttyS0" \
	-drive file=$(OUTPUT_DIR)/images/rootfs.ext4,format=raw,id=hd0 \
	-device virtio-blk-device,drive=hd0 \
	-netdev user,id=net0 -device virtio-net-device,netdev=net0 \
	-serial mon:stdio \
	-nographic

# === Targets ===

.PHONY: validate-kernel-config
validate-kernel-config:
	@if [ ! -f $(KERNEL_CONFIG) ]; then \
		echo "ERROR: Kernel config file missing for architecture '$(ARCH)': $(KERNEL_CONFIG)"; \
		exit 1; \
	fi

.PHONY: validate
validate: validate-kernel-config
	@if [ ! -f $(CONFIG_FILE) ]; then \
		echo "ERROR: Architecture '$(ARCH)' defconfig missing: $(CONFIG_FILE)"; \
		echo "Valid options are:"; \
		ls depends/configs/NexisOS_*_defconfig | sed 's/.*NexisOS_\(.*\)_defconfig/\1/' | xargs -n1 echo " -"; \
		exit 1; \
	fi

.PHONY: prepare
prepare: validate
	@echo "Ready to configure Buildroot for $(ARCH)"

.PHONY: copy-kernel-config
copy-kernel-config: validate-kernel-config
	@mkdir -p $(BUILDROOT_DIR)/board/nexisos
	@cp $(KERNEL_CONFIG) $(BUILDROOT_DIR)/board/nexisos/linux-$(ARCH).config
	@echo "Copied kernel config for $(ARCH) to Buildroot board/nexisos"

.PHONY: copy-nexpm-package
copy-nexpm-package:
	@mkdir -p $(BUILDROOT_DIR)/package/nexpm
	@cp depends/package/nexpm/nexpm.mk $(BUILDROOT_DIR)/package/nexpm/
	@cp depends/package/nexpm/Config.in $(BUILDROOT_DIR)/package/nexpm/
	@echo "Copied nexpm package files to Buildroot package directory"

.PHONY: patch-config
patch-config:
	@if ! grep -Fq "# Begin nexpm Config.in patch" $(BUILDROOT_CONFIG) 2>/dev/null; then \
		set -e; \
		if [ ! -f $(BACKUP_CONFIG) ]; then \
			cp $(BUILDROOT_CONFIG) $(BACKUP_CONFIG); \
		fi; \
		echo "# Begin nexpm Config.in patch" >> $(BUILDROOT_CONFIG); \
		cat depends/package/Config.in >> $(BUILDROOT_CONFIG); \
		echo "# End nexpm Config.in patch" >> $(BUILDROOT_CONFIG); \
		echo "Appended nexpm Config.in to Buildroot package/Config.in"; \
	else \
		echo "nexpm Config.in patch already applied to Buildroot package/Config.in"; \
	fi

.PHONY: copy-runtime-files
copy-runtime-files:
	@mkdir -p $(BUILDROOT_DIR)/board/nexisos/rootfs-overlay/root/package_manager
	@mkdir -p $(BUILDROOT_DIR)/board/nexisos/rootfs-overlay/root/scripts
	@cp -r depends/package_manager/* $(BUILDROOT_DIR)/board/nexisos/rootfs-overlay/root/package_manager/
	@cp depends/scripts/*.sh $(BUILDROOT_DIR)/board/nexisos/rootfs-overlay/root/scripts/
	@chmod +x $(BUILDROOT_DIR)/board/nexisos/rootfs-overlay/root/scripts/*.sh
	@echo "📦 Copied package manager and scripts to rootfs overlay"

.PHONY: setup-postbuild
setup-postbuild:
	@mkdir -p $(BUILDROOT_DIR)/board/nexisos
	@cat > $(BUILDROOT_DIR)/board/nexisos/post-build.sh << 'EOF'
#!/bin/sh
# Run install.sh at login
echo "/root/scripts/install.sh" >> $(TARGET_DIR)/etc/profile
EOF
	@chmod +x $(BUILDROOT_DIR)/board/nexisos/post-build.sh
	@echo "🛠️  Created post-build hook to auto-launch installer on boot"

.PHONY: copy-dependencies
copy-dependencies: copy-kernel-config copy-nexpm-package patch-config copy-runtime-files setup-postbuild
	@echo "✅ All required dependencies and scripts prepared for Buildroot."

.PHONY: restore-config
restore-config:
	@if [ -f $(BACKUP_CONFIG) ]; then \
		mv $(BACKUP_CONFIG) $(BUILDROOT_CONFIG); \
		echo "Restored original Buildroot package/Config.in"; \
	fi

.PHONY: cleanup
cleanup:
	@rm -rf $(BUILDROOT_DIR)/package/nexpm
	@$(MAKE) restore-config
	@echo "Cleaned up copied nexpm package files and restored Buildroot config"

.PHONY: build
build: prepare copy-dependencies
	@echo "Building NexisOS ISO for $(ARCH)..."
	$(MAKE) -C $(BUILDROOT_DIR) O=$(OUTPUT_DIR) BR2_DEFCONFIG=$(CONFIG_FILE) defconfig -j$(shell nproc)
	$(MAKE) -C $(BUILDROOT_DIR) O=$(OUTPUT_DIR) -j$(shell nproc)
	@mkdir -p buildroot_backup_imgs/$(ARCH)/output/images
	@cp -r $(OUTPUT_DIR)/images/* buildroot_backup_imgs/$(ARCH)/output/images/
	$(MAKE) cleanup
	@echo "✅ Build complete. ISO and images copied to buildroot_backup_imgs/$(ARCH)/output/images"

.PHONY: clean
clean:
	@if [ -d $(BUILDROOT_DIR) ]; then \
		$(MAKE) -C $(BUILDROOT_DIR) O=$(OUTPUT_DIR) clean; \
	fi

.PHONY: distclean
distclean: clean
	@rm -rf $(OUTPUT_DIR) $(BACKUP_CONFIG)
	@echo "Removed output directory and backup config"

.PHONY: run-qemu
run-qemu:
	@if [ "$(ARCH)" = "x86_64" ]; then \
		echo "Running QEMU for x86_64..."; \
		$(QEMU_x86_64); \
	elif [ "$(ARCH)" = "aarch64" ]; then \
		echo "Running QEMU for aarch64..."; \
		$(QEMU_aarch64); \
	elif [ "$(ARCH)" = "riscv64" ]; then \
		echo "Running QEMU for riscv64..."; \
		$(QEMU_riscv64); \
	else \
		echo "Unsupported ARCH=$(ARCH) for run-qemu"; exit 1; \
	fi

.PHONY: help
help:
	@echo "NexisOS Makefile Commands:"
	@echo ""
	@echo "  make [ARCH=arch]     Build image for specified arch (default: x86_64)"
	@echo "  make clean           Clean build output for selected arch"
	@echo "  make distclean       Remove output directory for selected arch and backup config"
	@echo "  make run-qemu        Run NexisOS in QEMU with proper flags for the selected arch"
	@echo ""
	@echo "Available architectures:"
	@ls depends/configs/NexisOS_*_defconfig | sed 's/.*NexisOS_\(.*\)_defconfig/\1/' | xargs -n1 echo " -"
