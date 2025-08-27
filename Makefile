.DEFAULT_GOAL := build
.ONESHELL:

# === Variables ===

ARCH ?= x86_64

AVAILABLE_ARCHS := $(shell ls IsoDependencies/configs/NexisOS_*_defconfig | sed 's/.*NexisOS_\(.*\)_defconfig/\1/')

# Validate ARCH early
ifeq (,$(filter $(ARCH),$(AVAILABLE_ARCHS)))
$(error Unsupported ARCH=$(ARCH). Supported: $(AVAILABLE_ARCHS))
endif

DEFCONFIG := NexisOS_$(ARCH)_defconfig
CONFIG_FILE := $(abspath IsoDependencies/configs/$(DEFCONFIG))
KERNEL_CONFIG := $(abspath IsoDependencies/kernel-configs/linux-$(ARCH).config)

BUILDROOT_VERSION := 2025.xx.xx
BUILDROOT_DIR := ../buildroot
OUTPUT_DIR := output-$(ARCH)

BACKUP_CONFIG := $(BUILDROOT_DIR)/package/Config.in.bak
BUILDROOT_CONFIG := $(BUILDROOT_DIR)/package/Config.in

NEXISOS_BOARD_DIR := $(BUILDROOT_DIR)/board/nexisos

NUM_JOBS := $(shell nproc)

PATCH_MARKER := "# Begin nexpm Config.in patch"
PATCH_APPLIED := $(shell grep -Fq "$(PATCH_MARKER)" $(BUILDROOT_CONFIG) 2>/dev/null && echo yes || echo no)

ROOTFS_OVERLAY := $(NEXISOS_BOARD_DIR)/rootfs-overlay/root

# === Helper macros ===

define copy_with_mkdir
	@mkdir -p $(dir $(2))
	@cp -r $(1) $(2)
endef

define backup_images
	@mkdir -p buildroot_backup_imgs/$(ARCH)/output/images
	@cp -r $(OUTPUT_DIR)/images/* buildroot_backup_imgs/$(ARCH)/output/images/
endef

# === Targets ===

.PHONY: validate-kernel-config
validate-kernel-config:
	if [ ! -f $(KERNEL_CONFIG) ]; then
		echo "ERROR: Kernel config file missing for architecture '$(ARCH)': $(KERNEL_CONFIG)"
		exit 1
	fi

.PHONY: validate
validate: validate-kernel-config
	if [ ! -f $(CONFIG_FILE) ]; then
		echo "ERROR: Architecture '$(ARCH)' defconfig missing: $(CONFIG_FILE)"
		echo "Valid options are:"
		$(foreach arch,$(AVAILABLE_ARCHS),echo " - $(arch)")
		exit 1
	fi

.PHONY: prepare
prepare: validate
	@echo "Ready to configure Buildroot for $(ARCH)"

.PHONY: copy-kernel-config
copy-kernel-config: validate-kernel-config
	@mkdir -p $(NEXISOS_BOARD_DIR)
	@install -m 644 $(KERNEL_CONFIG) $(NEXISOS_BOARD_DIR)/linux-$(ARCH).config
	@echo "Copied kernel config for $(ARCH) to Buildroot board/nexisos"

.PHONY: copy-nexpm-package
copy-nexpm-package:
	@mkdir -p $(BUILDROOT_DIR)/package/nexpm
	@install -m 644 IsoDependencies/package/nexpm/nexpm.mk $(BUILDROOT_DIR)/package/nexpm/
	@install -m 644 IsoDependencies/package/nexpm/Config.in $(BUILDROOT_DIR)/package/nexpm/
	@echo "Copied nexpm package files to Buildroot package directory"

.PHONY: patch-config
patch-config:
ifneq ($(PATCH_APPLIED),yes)
	@if [ ! -f $(BACKUP_CONFIG) ]; then cp $(BUILDROOT_CONFIG) $(BACKUP_CONFIG); fi
	@echo "$(PATCH_MARKER)" >> $(BUILDROOT_CONFIG)
	@cat IsoDependencies/package/Config.in >> $(BUILDROOT_CONFIG)
	@echo "# End nexpm Config.in patch" >> $(BUILDROOT_CONFIG)
	@echo "Appended nexpm Config.in to Buildroot package/Config.in"
else
	@echo "nexpm Config.in patch already applied to Buildroot package/Config.in"
endif

.PHONY: copy-runtime-files
copy-runtime-files:
	$(call copy_with_mkdir,IsoDependencies/package_manager/*,$(ROOTFS_OVERLAY)/package_manager/)
	$(call copy_with_mkdir,IsoDependencies/scripts/*.sh,$(ROOTFS_OVERLAY)/scripts/)
	@chmod +x $(ROOTFS_OVERLAY)/scripts/*.sh
	@echo "📦 Copied package manager and scripts to rootfs overlay"

.PHONY: copy-overlay
copy-overlay:
	@rm -rf $(NEXISOS_BOARD_DIR)/rootfs-overlay
	@rsync -a --delete IsoDependencies/overlay/ $(NEXISOS_BOARD_DIR)/rootfs-overlay/
	@echo "Copied overlay files to Buildroot board/nexisos/rootfs-overlay"

.PHONY: setup-postbuild
setup-postbuild:
	@mkdir -p $(NEXISOS_BOARD_DIR)
	@cat > $(NEXISOS_BOARD_DIR)/post-build.sh << 'EOF'
#!/bin/sh
# Run install.sh at login
echo "/root/scripts/install.sh" >> $(TARGET_DIR)/etc/profile
EOF
	@chmod +x $(NEXISOS_BOARD_DIR)/post-build.sh
	@echo "🛠️  Created post-build hook to auto-launch installer on boot"

.PHONY: prepare-deps
prepare-deps: copy-kernel-config copy-nexpm-package patch-config copy-runtime-files copy-overlay setup-postbuild
	@echo "✅ All required dependencies and overlays prepared for Buildroot."

.PHONY: restore-config
restore-config:
	if [ -f $(BACKUP_CONFIG) ]; then
		mv $(BACKUP_CONFIG) $(BUILDROOT_CONFIG)
		echo "Restored original Buildroot package/Config.in"
	fi

.PHONY: cleanup
cleanup:
	@rm -rf $(BUILDROOT_DIR)/package/nexpm
	@$(MAKE) restore-config
	@echo "Cleaned up copied nexpm package files and restored Buildroot config"

.PHONY: build
build: prepare prepare-deps
	@echo "Building NexisOS ISO for $(ARCH)..."
	$(MAKE) -C $(BUILDROOT_DIR) O=$(OUTPUT_DIR) BR2_DEFCONFIG=$(CONFIG_FILE) defconfig
	$(MAKE) -C $(BUILDROOT_DIR) O=$(OUTPUT_DIR) -j$(NUM_JOBS)
	$(call backup_images)
	$(MAKE) cleanup
	@echo "✅ Build complete. ISO and images copied to buildroot_backup_imgs/$(ARCH)/output/images"

.PHONY: clean
clean:
	if [ -d $(BUILDROOT_DIR) ]; then
		$(MAKE) -C $(BUILDROOT_DIR) O=$(OUTPUT_DIR) clean
	fi

.PHONY: distclean
distclean: clean
	@rm -rf $(OUTPUT_DIR) $(BACKUP_CONFIG)
	@echo "Removed output directory and backup config"

.PHONY: run-qemu
run-qemu:
	@echo "🖥️  Launching QEMU for $(ARCH)..."
	@ARCH_SCRIPT=MakeDependacies/scripts/qemu/run_$(ARCH).sh; \
	if [ ! -f "$$ARCH_SCRIPT" ]; then \
		echo "❌ QEMU script not found: $$ARCH_SCRIPT"; \
		exit 1; \
	fi; \
	chmod +x $$ARCH_SCRIPT; \
	exec "$$ARCH_SCRIPT"

.PHONY: list-archs
list-archs:
	@echo "Available architectures:"
	@$(foreach arch,$(AVAILABLE_ARCHS),echo " - $(arch)")

.PHONY: help
help:
	@echo "NexisOS Makefile Commands:"
	@echo ""
	@echo "  make [ARCH=arch]     Build image for specified arch (default: x86_64)"
	@echo "  make clean           Clean build output for selected arch"
	@echo "  make distclean       Full cleanup"
	@echo "  make run-qemu        Run the built image in QEMU"
	@echo "  make list-archs      Show supported architectures"
