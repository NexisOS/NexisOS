.DEFAULT_GOAL := menu
.ONESHELL:

# === Variables ===
BUILDROOT_DIR := buildroot
CONFIGS_DIR := distroConfigs
OUTPUT_DIR := createdISOs
SCRIPTS_DIR := $(CONFIGS_DIR)/scripts
PACKAGES_DIR := $(CONFIGS_DIR)/packages
OVERLAY_DIR := $(CONFIGS_DIR)/overlay
BR_PACKAGES_DIR := $(CONFIGS_DIR)/buildroot-packages
NUM_JOBS := $(shell nproc)

# Buildroot directories
BR_BOARD_DIR := $(BUILDROOT_DIR)/board/nexisos
BR_PACKAGE_DIR := $(BUILDROOT_DIR)/package
BR_DL_DIR := $(BUILDROOT_DIR)/dl

# === Helper Functions ===
define copy_with_mkdir
	@mkdir -p $(dir $(2))
	@cp -r $(1) $(2)
endef

define setup_buildroot_dirs
	@echo "📁 Setting up Buildroot directories..."
	@mkdir -p $(BR_BOARD_DIR)/rootfs-overlay
	@mkdir -p $(BR_PACKAGE_DIR)
	@mkdir -p $(BR_DL_DIR)
endef

define copy_defconfig
	@echo "📄 Copying Buildroot defconfig..."
	@if [ -f "$(CONFIGS_DIR)/NexisOS_defonfig" ]; then \
		cp $(CONFIGS_DIR)/NexisOS_defonfig $(BUILDROOT_DIR)/configs/NexisOS_defconfig; \
		echo "  ✅ Defconfig copied"; \
	else \
		echo "  ❌ NexisOS_defonfig not found in $(CONFIGS_DIR)"; \
		exit 1; \
	fi
endef

define copy_overlay
	@echo "📦 Copying overlay files..."
	@if [ -d "$(OVERLAY_DIR)" ]; then \
		rsync -a --delete $(OVERLAY_DIR)/ $(BR_BOARD_DIR)/rootfs-overlay/; \
		echo "  ✅ Overlay copied"; \
	else \
		echo "  ⚠️  No overlay directory found"; \
	fi
endef

define copy_scripts
	@echo "📜 Copying runtime scripts..."
	@mkdir -p $(BR_BOARD_DIR)/rootfs-overlay/scripts
	@if [ -d "$(SCRIPTS_DIR)" ] && [ -n "$$(ls -A $(SCRIPTS_DIR)/*.sh 2>/dev/null)" ]; then \
		cp $(SCRIPTS_DIR)/*.sh $(BR_BOARD_DIR)/rootfs-overlay/scripts/; \
		chmod +x $(BR_BOARD_DIR)/rootfs-overlay/scripts/*.sh; \
		echo "  ✅ Scripts copied and made executable"; \
	else \
		echo "  ⚠️  No shell scripts found in $(SCRIPTS_DIR)"; \
	fi
endef

define copy_buildroot_packages
	@echo "📦 Copying Buildroot package definitions..."
	@if [ -d "$(BR_PACKAGES_DIR)" ]; then \
		for pkg_dir in $(BR_PACKAGES_DIR)/*; do \
			if [ -d "$$pkg_dir" ]; then \
				pkg_name=$$(basename $$pkg_dir); \
				mkdir -p $(BR_PACKAGE_DIR)/$$pkg_name; \
				rsync -a --delete $$pkg_dir/ $(BR_PACKAGE_DIR)/$$pkg_name/; \
				echo "  ✅ Copied package definition: $$pkg_name"; \
			fi; \
		done; \
	else \
		echo "  ⚠️  No buildroot-packages directory found"; \
	fi
endef

define copy_rust_source
	@echo "📦 Copying Rust workspace source to Buildroot dl directory..."
	@mkdir -p $(BR_DL_DIR)
	@for pkg in nexis_init nexis_pm nexis_scan; do \
		if [ -d "$(PACKAGES_DIR)/$$pkg" ]; then \
			mkdir -p $(BR_DL_DIR)/$$pkg; \
			rsync -a --delete \
				--exclude 'target' \
				--exclude '.git' \
				--exclude '*.swp' \
				--exclude '.gitignore' \
				$(PACKAGES_DIR)/$$pkg/ $(BR_DL_DIR)/$$pkg/; \
			echo "  ✅ Copied $$pkg source"; \
		else \
			echo "  ⚠️  Package $$pkg not found"; \
		fi; \
	done
	@if [ -d "$(PACKAGES_DIR)/nexis_common" ]; then \
		mkdir -p $(BR_DL_DIR)/nexis_common; \
		rsync -a --delete \
			--exclude 'target' \
			--exclude '.git' \
			$(PACKAGES_DIR)/nexis_common/ $(BR_DL_DIR)/nexis_common/; \
		echo "  ✅ Copied nexis_common (library) source"; \
	fi
	@if [ -f "$(PACKAGES_DIR)/Cargo.toml" ]; then \
		cp $(PACKAGES_DIR)/Cargo.toml $(BR_DL_DIR)/; \
		echo "  ✅ Copied workspace Cargo.toml"; \
	fi
endef

# === Clean Buildroot (reset submodule) ===
.PHONY: clean-buildroot
clean-buildroot:
	@echo "🧹 Cleaning Buildroot and restoring submodule to pristine state..."
	@cd $(BUILDROOT_DIR) && git reset --hard && git clean -fdx
	@echo "✅ Buildroot reset complete."

# === Prepare Buildroot ===
.PHONY: prepare
prepare:
	@echo "🔧 Preparing Buildroot environment..."
	$(call setup_buildroot_dirs)
	$(call copy_defconfig)
	$(call copy_overlay)
	$(call copy_scripts)
	$(call copy_buildroot_packages)
	$(call copy_rust_source)
	@echo "✅ Buildroot preparation complete."

# === Build Rules ===
.PHONY: build
build: prepare
	@echo "🚀 Building NexisOS..."
	@set -e; \
	trap 'echo "❌ Build failed. Run '\''make clean-buildroot'\'' to reset."; exit 1' ERR; \
	make -C $(BUILDROOT_DIR) BR2_DEFCONFIG=$(BUILDROOT_DIR)/configs/NexisOS_defconfig O=$(OUTPUT_DIR) defconfig; \
	make -C $(BUILDROOT_DIR) O=$(OUTPUT_DIR) -j$(NUM_JOBS); \
	ISO_PATH=$(OUTPUT_DIR)/images/*.iso; \
	if compgen -G "$$ISO_PATH" > /dev/null; then \
		for iso in $$ISO_PATH; do \
			cp $$iso $(OUTPUT_DIR)/NexisOS.iso; \
			echo "✅ ISO created: $(OUTPUT_DIR)/NexisOS.iso"; \
		done; \
	else \
		echo "❌ ISO not found in $(OUTPUT_DIR)/images/"; \
		exit 1; \
	fi

# === Quick rebuild (without cleaning) ===
.PHONY: rebuild
rebuild:
	@echo "🔄 Rebuilding NexisOS (without prepare)..."
	@make -C $(BUILDROOT_DIR) O=$(OUTPUT_DIR) -j$(NUM_JOBS)

# === QEMU Rules ===
.PHONY: qemu
qemu:
	@echo "🖥️  Launching QEMU with GUI"
	@if [ ! -f "$(OUTPUT_DIR)/NexisOS.iso" ]; then \
		echo "❌ ISO not found. Build one first with 'make build'"; \
		exit 1; \
	fi; \
	ARCH=$$(whiptail --title "Select Architecture" --menu "Choose architecture for QEMU:" 15 60 5 \
		"x86_64" "Intel/AMD 64-bit" \
		"aarch64" "ARM 64-bit" \
		"riscv64" "RISC-V 64-bit" \
		3>&1 1>&2 2>&3); \
	if [ -z "$$ARCH" ]; then \
		echo "⚠️  No architecture selected. Canceled."; \
		exit 0; \
	fi; \
	RAM=$$(whiptail --inputbox "Enter RAM size for QEMU in MB (default 2048):" 10 50 2048 3>&1 1>&2 2>&3 || echo "2048"); \
	if [ -z "$$RAM" ]; then RAM=2048; fi; \
	CORES=$$(whiptail --inputbox "Enter number of CPU cores (default 4):" 10 50 4 3>&1 1>&2 2>&3 || echo "4"); \
	if [ -z "$$CORES" ]; then CORES=4; fi; \
	case "$$ARCH" in \
		x86_64) \
			echo "Launching QEMU x86_64 with $$RAM MB RAM and $$CORES cores..."; \
			qemu-system-x86_64 \
				-cdrom $(OUTPUT_DIR)/NexisOS.iso \
				-m $$RAM \
				-smp $$CORES \
				-boot d \
				-vga std \
				-enable-kvm \
				& disown ;; \
		aarch64) \
			echo "Launching QEMU aarch64 with $$RAM MB RAM and $$CORES cores..."; \
			qemu-system-aarch64 \
				-M virt \
				-cpu cortex-a57 \
				-cdrom $(OUTPUT_DIR)/NexisOS.iso \
				-m $$RAM \
				-smp $$CORES \
				-boot d \
				-device virtio-gpu-pci \
				-device qemu-xhci \
				-device usb-kbd \
				-device usb-mouse \
				& disown ;; \
		riscv64) \
			echo "Launching QEMU riscv64 with $$RAM MB RAM and $$CORES cores..."; \
			qemu-system-riscv64 \
				-M virt \
				-cdrom $(OUTPUT_DIR)/NexisOS.iso \
				-m $$RAM \
				-smp $$CORES \
				-boot d \
				-device virtio-gpu-pci \
				-device qemu-xhci \
				-device usb-kbd \
				& disown ;; \
		*) \
			echo "❌ Unsupported architecture: $$ARCH"; \
			exit 1 ;; \
	esac

# === QEMU headless (for testing) ===
.PHONY: qemu-headless
qemu-headless:
	@echo "🖥️  Launching QEMU in headless mode..."
	@if [ ! -f "$(OUTPUT_DIR)/NexisOS.iso" ]; then \
		echo "❌ ISO not found. Build one first with 'make build'"; \
		exit 1; \
	fi; \
	ARCH=$$(whiptail --title "Select Architecture" --menu "Choose architecture for QEMU:" 15 60 5 \
		"x86_64" "Intel/AMD 64-bit" \
		"aarch64" "ARM 64-bit" \
		"riscv64" "RISC-V 64-bit" \
		3>&1 1>&2 2>&3); \
	if [ -z "$$ARCH" ]; then \
		echo "⚠️  No architecture selected. Canceled."; \
		exit 0; \
	fi; \
	case "$$ARCH" in \
		x86_64) \
			qemu-system-x86_64 \
				-cdrom $(OUTPUT_DIR)/NexisOS.iso \
				-m 2048 \
				-smp 2 \
				-boot d \
				-nographic \
				-serial mon:stdio ;; \
		aarch64) \
			qemu-system-aarch64 \
				-M virt \
				-cpu cortex-a57 \
				-cdrom $(OUTPUT_DIR)/NexisOS.iso \
				-m 2048 \
				-smp 2 \
				-boot d \
				-nographic \
				-serial mon:stdio ;; \
		riscv64) \
			qemu-system-riscv64 \
				-M virt \
				-cdrom $(OUTPUT_DIR)/NexisOS.iso \
				-m 2048 \
				-smp 2 \
				-boot d \
				-nographic \
				-serial mon:stdio ;; \
		*) \
			echo "❌ Unsupported architecture: $$ARCH"; \
			exit 1 ;; \
	esac

# === Development: sync changes without full rebuild ===
.PHONY: sync
sync:
	@echo "🔄 Syncing configuration changes to Buildroot..."
	$(call copy_overlay)
	$(call copy_scripts)
	$(call copy_buildroot_packages)
	$(call copy_rust_source)
	@echo "✅ Sync complete. Run 'make rebuild' to build with changes."

# === Menu Using Whiptail ===
.PHONY: menu
menu:
	@CHOICE=$$(whiptail --title "NexisOS Build Menu" --menu "Select action:" 22 65 12 \
		"1" "Build ISO (full build with prepare)" \
		"2" "Rebuild (quick rebuild without prepare)" \
		"3" "Sync changes only (no build)" \
		"4" "Run ISO in QEMU (GUI)" \
		"5" "Run ISO in QEMU (headless)" \
		"6" "Check ISO status" \
		"7" "Show configuration" \
		"8" "Clean Buildroot (reset to pristine)" \
		"9" "Clean output directory" \
		3>&1 1>&2 2>&3); \
	case $$CHOICE in \
		1) make build ;; \
		2) make rebuild ;; \
		3) make sync ;; \
		4) make qemu ;; \
		5) make qemu-headless ;; \
		6) make check-iso ;; \
		7) make show-config ;; \
		8) make clean-buildroot ;; \
		9) make clean ;; \
		*) echo "⚠️  Canceled."; exit 0 ;; \
	esac

# === Check if ISO exists ===
.PHONY: check-iso
check-iso:
	@if [ -f "$(OUTPUT_DIR)/NexisOS.iso" ]; then \
		echo "✅ ISO found: $(OUTPUT_DIR)/NexisOS.iso"; \
		ls -lh $(OUTPUT_DIR)/NexisOS.iso; \
	else \
		echo "❌ No ISO found. Run 'make build' first."; \
	fi

# === Cleanup ===
.PHONY: clean
clean:
	@echo "🧹 Cleaning output directory..."
	@rm -rf $(OUTPUT_DIR)/*
	@echo "✅ Output directory cleaned."

.PHONY: clean-all
clean-all: clean clean-buildroot
	@echo "✅ Full cleanup complete."

# === Show current configuration ===
.PHONY: show-config
show-config:
	@echo "📋 NexisOS Configuration:"
	@echo "  Buildroot dir:     $(BUILDROOT_DIR)"
	@echo "  Configs dir:       $(CONFIGS_DIR)"
	@echo "  Output dir:        $(OUTPUT_DIR)"
	@echo "  Scripts dir:       $(SCRIPTS_DIR)"
	@echo "  Packages dir:      $(PACKAGES_DIR)"
	@echo "  Overlay dir:       $(OVERLAY_DIR)"
	@echo "  BR packages dir:   $(BR_PACKAGES_DIR)"
	@echo "  CPU cores:         $(NUM_JOBS)"
	@echo ""
	@echo "📦 Rust packages (source):"
	@ls -1 $(PACKAGES_DIR) | grep -v Cargo.toml || echo "  None found"
	@echo ""
	@echo "📦 Buildroot package definitions:"
	@if [ -d "$(BR_PACKAGES_DIR)" ]; then \
		ls -1 $(BR_PACKAGES_DIR) || echo "  None found"; \
	else \
		echo "  Directory not found"; \
	fi

# === Help ===
.PHONY: help
help:
	@echo "NexisOS Makefile - Available targets:"
	@echo ""
	@echo "Building:"
	@echo "  make menu          - Interactive menu (recommended)"
	@echo "  make build         - Full build: prepare + compile + create ISO"
	@echo "  make rebuild       - Quick rebuild without preparation step"
	@echo "  make prepare       - Only prepare Buildroot (copy configs/packages)"
	@echo ""
	@echo "Development:"
	@echo "  make sync          - Sync changes to Buildroot without building"
	@echo "  make qemu          - Run ISO in QEMU with GUI (select architecture)"
	@echo "  make qemu-headless - Run ISO in QEMU headless (select architecture)"
	@echo ""
	@echo "Maintenance:"
	@echo "  make clean         - Remove output files only"
	@echo "  make clean-buildroot - Reset Buildroot submodule to pristine state"
	@echo "  make clean-all     - Clean everything"
	@echo ""
	@echo "Information:"
	@echo "  make check-iso     - Check if ISO exists and show details"
	@echo "  make show-config   - Display current configuration"
	@echo "  make help          - Show this help message"
