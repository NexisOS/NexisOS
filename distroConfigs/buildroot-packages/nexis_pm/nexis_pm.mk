################################################################################
#
# nexis_pm
#
################################################################################

NEXIS_PM_VERSION = 1.0.0
NEXIS_PM_SITE = $(TOPDIR)/../buildroot/dl/nexis_pm
NEXIS_PM_SITE_METHOD = local
NEXIS_PM_LICENSE = MIT OR Apache-2.0
NEXIS_PM_LICENSE_FILES = LICENSE-MIT LICENSE-APACHE

# Cargo package infrastructure
NEXIS_PM_CARGO_MODE = release

# Dependencies
NEXIS_PM_DEPENDENCIES = host-rustc nexis_common

# Set up Cargo environment
NEXIS_PM_CARGO_ENV = \
	CARGO_HOME=$(HOST_DIR)/share/cargo \
	RUSTFLAGS="-C target-feature=+crt-static"

define NEXIS_PM_BUILD_CMDS
	cd $(@D) && \
	$(NEXIS_PM_CARGO_ENV) \
	$(HOST_DIR)/bin/cargo build \
		--release \
		--target=$(RUSTC_TARGET_NAME) \
		--manifest-path=$(@D)/Cargo.toml
endef

define NEXIS_PM_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 \
		$(@D)/target/$(RUSTC_TARGET_NAME)/release/nexis_pm \
		$(TARGET_DIR)/usr/bin/nexis_pm
endef

$(eval $(generic-package))
