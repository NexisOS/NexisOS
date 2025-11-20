################################################################################
#
# nexis_init
#
################################################################################

NEXIS_INIT_VERSION = 1.0.0
NEXIS_INIT_SITE = $(TOPDIR)/../buildroot/dl/nexis_init
NEXIS_INIT_SITE_METHOD = local
NEXIS_INIT_LICENSE = MIT OR Apache-2.0
NEXIS_INIT_LICENSE_FILES = LICENSE-MIT LICENSE-APACHE

# Cargo package infrastructure
NEXIS_INIT_CARGO_MODE = release

# Dependencies
NEXIS_INIT_DEPENDENCIES = host-rustc nexis_common

# Set up Cargo environment
NEXIS_INIT_CARGO_ENV = \
	CARGO_HOME=$(HOST_DIR)/share/cargo \
	RUSTFLAGS="-C target-feature=+crt-static"

define NEXIS_INIT_BUILD_CMDS
	cd $(@D) && \
	$(NEXIS_INIT_CARGO_ENV) \
	$(HOST_DIR)/bin/cargo build \
		--release \
		--target=$(RUSTC_TARGET_NAME) \
		--manifest-path=$(@D)/Cargo.toml
endef

define NEXIS_INIT_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 \
		$(@D)/target/$(RUSTC_TARGET_NAME)/release/nexis_init \
		$(TARGET_DIR)/sbin/nexis_init
endef

$(eval $(generic-package))
