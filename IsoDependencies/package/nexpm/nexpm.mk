NEXPM_VERSION = 0.1.0-pre-alpha
NEXPM_SITE = $(TOPDIR)/depends/package_manager
NEXPM_SITE_METHOD = local
NEXPM_DEPENDENCIES = host-rustc

NEXPM_CARGO_ENV = \
	CARGO_HOME=$(@D)/.cargo \
	RUSTFLAGS="-C opt-level=z -C linker-plugin-lto -C strip=symbols"

NEXPM_RUST_TARGET = x86_64-unknown-linux-musl

define NEXPM_BUILD_CMDS
	$(NEXPM_CARGO_ENV) \
	cargo build --release \
		--target=$(NEXPM_RUST_TARGET) \
		--no-default-features --features="cli,network,parallel" \
		--manifest-path $(@D)/Cargo.toml
endef

define NEXPM_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 \
		$(@D)/target/$(NEXPM_RUST_TARGET)/release/nexpm \
		$(TARGET_DIR)/usr/bin/nexpm
endef

$(eval $(generic-package))
