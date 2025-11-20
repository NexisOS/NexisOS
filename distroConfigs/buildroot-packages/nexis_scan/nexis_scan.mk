################################################################################
#
# nexis_scan
#
################################################################################

NEXIS_SCAN_VERSION = 1.0.0
NEXIS_SCAN_SITE = $(TOPDIR)/../buildroot/dl/nexis_scan
NEXIS_SCAN_SITE_METHOD = local
NEXIS_SCAN_LICENSE = MIT OR Apache-2.0
NEXIS_SCAN_LICENSE_FILES = LICENSE-MIT LICENSE-APACHE

# Cargo package infrastructure
NEXIS_SCAN_CARGO_MODE = release

# Dependencies
NEXIS_SCAN_DEPENDENCIES = host-rustc nexis_common

# Set up Cargo environment
NEXIS_SCAN_CARGO_ENV = \
	CARGO_HOME=$(HOST_DIR)/share/cargo \
	RUSTFLAGS="-C target-feature=+crt-static"

define NEXIS_SCAN_BUILD_CMDS
	cd $(@D) && \
	$(NEXIS_SCAN_CARGO_ENV) \
	$(HOST_DIR)/bin/cargo build \
		--release \
		--target=$(RUSTC_TARGET_NAME) \
		--manifest-path=$(@D)/Cargo.toml
endef

define NEXIS_SCAN_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 \
		$(@D)/target/$(RUSTC_TARGET_NAME)/release/nexis_scan \
		$(TARGET_DIR)/usr/bin/nexis_scan
endef

$(eval $(generic-package))
