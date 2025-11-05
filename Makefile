.DEFAULT_GOAL := menu
.ONESHELL:

# === Variables ===
BUILDROOT_DIR := buildroot
CONFIGS_DIR := distroConfigs
OUTPUT_DIR := createdISOs
SCRIPTS_DIR := $(CONFIGS_DIR)/scripts
NUM_JOBS := $(shell nproc)

ARCHS := aarch64 x86_64 riscv64

# === Helper Functions ===
define copy_with_mkdir
	@mkdir -p $(dir $(2))
	@cp -r $(1) $(2)
endef

define copy_config_and_overlay
	@echo "📄 Copying Buildroot defconfig, kernel config, and overlay for $(1)..."
	cp $(CONFIGS_DIR)/$(1)/NexisOS_$(1)_defconfig $(BUILDROOT_DIR)/configs/
	if [ -f "$(CONFIGS_DIR)/$(1)/linux-$(1).config" ]; then \
		cp $(CONFIGS_DIR)/$(1)/linux-$(1).config $(BUILDROOT_DIR)/board/nexisos/linux.config; \
	fi
	rm -rf $(BUILDROOT_DIR)/board/nexisos/rootfs-overlay/*
	rsync -a --delete $(CONFIGS_DIR)/$(1)/overlay/ $(BUILDROOT_DIR)/board/nexisos/rootfs-overlay/
endef

define copy_scripts_and_packages
	@echo "📦 Copying runtime scripts..."
	rm -rf $(BUILDROOT_DIR)/board/nexisos/rootfs-overlay/scripts/*
	$(call copy_with_mkdir,$(SCRIPTS_DIR)/*.sh,$(BUILDROOT_DIR)/board/nexisos/rootfs-overlay/scripts/)
	chmod +x $(BUILDROOT_DIR)/board/nexisos/rootfs-overlay/scripts/*.sh

	@echo "📦 Copying all packages..."
	rm -rf $(BUILDROOT_DIR)/board/nexisos/rootfs-overlay/packages/*
	@for pkg in $(CONFIGS_DIR)/packages/*; do \
		if [ -d "$$pkg" ]; then \
			dest=$(BUILDROOT_DIR)/board/nexisos/rootfs-overlay/packages/$$(basename $$pkg); \
			mkdir -p "$$dest"; \
			rsync -a --delete "$$pkg/" "$$dest/"; \
			echo "  ✅ Copied package $$(basename $$pkg)"; \
		fi; \
	done
endef

# === Clean Buildroot (reset submodule) ===
.PHONY: clean-buildroot
clean-buildroot:
	@echo "🧹 Cleaning Buildroot output and restoring submodule..."
	rm -rf $(OUTPUT_DIR)/*
	cd $(BUILDROOT_DIR) && git reset --hard && git clean -fdx
	@echo "✅ Buildroot reset complete."

# === Build Rules ===
.PHONY: build
build:
	@ARCH=$${ARCH:-x86_64}; \
	if ! echo "$(ARCHS)" | grep -qw $$ARCH; then \
		echo "❌ Unsupported architecture: $$ARCH"; exit 1; \
	fi; \
	$(call copy_config_and_overlay,$$ARCH); \
	$(call copy_scripts_and_packages); \
	echo "🚀 Building NexisOS for $$ARCH..."; \
	set -e; \
	trap 'echo "❌ Build failed. Cleaning Buildroot..."; make clean-buildroot; exit 1' ERR; \
	make -C $(BUILDROOT_DIR) BR2_DEFCONFIG=$(BUILDROOT_DIR)/configs/NexisOS_$${ARCH}_defconfig O=$(OUTPUT_DIR)/$${ARCH} defconfig; \
	make -C $(BUILDROOT_DIR) O=$(OUTPUT_DIR)/$${ARCH} -j$(NUM_JOBS); \
	ISO_PATH=$(OUTPUT_DIR)/$${ARCH}/images/*.iso; \
	if compgen -G "$$ISO_PATH" > /dev/null; then \
		cp $$ISO_PATH $(OUTPUT_DIR)/NexisOS_$${ARCH}.iso; \
		echo "✅ ISO created: $(OUTPUT_DIR)/NexisOS_$${ARCH}.iso"; \
	else \
		echo "❌ ISO not found."; exit 1; \
	fi; \
	make clean-buildroot

# === QEMU Rules ===
.PHONY: qemu
qemu:
	@echo "🖥️  Launch QEMU with GUI"
	ISOS=$$(ls $(OUTPUT_DIR)/NexisOS_*.iso 2>/dev/null); \
	if [ -z "$$ISOS" ]; then \
		echo "❌ No ISOs found in $(OUTPUT_DIR). Build one first."; exit 1; \
	fi; \
	CHOICE=$$(whiptail --title "Select ISO to run" --menu "Choose ISO to run in QEMU (or Cancel to skip):" 20 60 10 \
	$$(for f in $$ISOS; do base=$$(basename $$f .iso); echo "$$base" "$$f"; done) 3>&1 1>&2 2>&3); \
	if [ -z "$$CHOICE" ]; then \
		echo "⚠️  No ISO selected. Skipping QEMU."; exit 0; \
	fi; \
	case "$$CHOICE" in \
		*NexisOS_aarch64) QEMU_SYS=qemu-system-aarch64 ;; \
		*NexisOS_x86_64) QEMU_SYS=qemu-system-x86_64 ;; \
		*NexisOS_riscv64) QEMU_SYS=qemu-system-riscv64 ;; \
		*) echo "❌ Could not determine QEMU system for $$CHOICE"; exit 1 ;; \
	esac; \
	RAM=$$(whiptail --inputbox "Enter RAM size for QEMU in MB (default 2048):" 10 50 2048 3>&1 1>&2 2>&3); \
	if [ -z "$$RAM" ]; then RAM=2048; fi; \
	echo "Launching QEMU for $$CHOICE ($$QEMU_SYS) with $$RAM MB RAM..."; \
	$$QEMU_SYS -cdrom $(OUTPUT_DIR)/$$CHOICE.iso -m $$RAM -boot d -vga std & disown

# === Menu Using Whiptail ===
.PHONY: menu
menu:
	@CHOICE=$$(whiptail --title "NexisOS Build Menu" --menu "Select action:" 20 60 10 \
	"1" "Build an ISO" \
	"2" "Run ISO in QEMU (optional)" 3>&1 1>&2 2>&3); \
	case $$CHOICE in \
		1) ARCH=$$(whiptail --title "Choose Architecture" --menu "Select architecture to build:" 15 60 5 \
			"$(ARCHS)" "$(ARCHS)" 3>&1 1>&2 2>&3); \
			if [ -n "$$ARCH" ]; then make build ARCH=$$ARCH; else echo "⚠️  Build canceled."; fi ;; \
		2) make qemu ;; \
		*) exit 0 ;; \
	esac

# === List available ISOs ===
.PHONY: list-isos
list-isos:
	@echo "Existing ISOs in $(OUTPUT_DIR):"
	@ls -1 $(OUTPUT_DIR)/NexisOS_*.iso 2>/dev/null || echo "No ISOs found"

# === Cleanup ===
.PHONY: clean
clean:
	@rm -rf $(OUTPUT_DIR)/*

# === Help ===
.PHONY: help
help:
	@echo "Usage:"
	@echo "  make menu            - Interactive menu with whiptail"
	@echo "  make build ARCH=ARCH - Build ISO for specified ARCH (aarch64, x86_64, riscv64)"
	@echo "  make qemu            - Run ISO in QEMU (with GUI and RAM selection)"
	@echo "  make list-isos       - List already-built ISOs"
	@echo "  make clean           - Remove all output files"
	@echo "  make clean-buildroot - Reset Buildroot submodule to pristine state"
