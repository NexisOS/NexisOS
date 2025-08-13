#!/bin/sh

set -e

# === Helper function for error handling ===
error_exit() {
    echo "Error: $1" >&2
    exit 1
}

log_info() {
    echo "[INFO] $1"
}

log_warn() {
    echo "[WARN] $1"
}

log_info "Starting NexisOS post-install script..."

# === 1. Set hostname ===
log_info "Setting hostname..."
echo "mydistro" > /etc/hostname

# === 2. Create default user ===
log_info "Creating default user..."
adduser -D myuser || error_exit "Failed to create user 'myuser'"
echo "myuser:changeme" | chpasswd || error_exit "Failed to set password for 'myuser'"

# === 3. Create system user for services ===
log_info "Creating service user 'nexisuser'..."
if ! id "nexisuser" >/dev/null 2>&1; then
    useradd -r -m -s /bin/bash nexisuser || error_exit "Failed to create 'nexisuser'"
fi

# === 4. Set up configuration directory ===
log_info "Setting up /etc/NexisOS..."
mkdir -p /etc/NexisOS || error_exit "Failed to create config directory"
chmod 755 /etc/NexisOS

# === 5. Copy default TOML config ===
log_info "Copying default config.toml..."
if [ -f "/usr/share/nexis/config.toml" ]; then
    cp /usr/share/nexis/config.toml /etc/NexisOS/config.toml || error_exit "Failed to copy configuration file"
    chmod 644 /etc/NexisOS/config.toml
else
    log_warn "Default config.toml not found. Skipping copy."
fi

# === 6. Install essential system packages ===
log_info "Installing core system packages with nexis-pkg..."
nexis-pkg install core-tools || error_exit "Failed to install 'core-tools'"

# === 7. Handle dependency resolution ===
log_info "Resolving package dependencies..."
nexis-pkg resolve-deps || error_exit "Dependency resolution failed"

# === 8. Install bootloader ===
log_info "Installing bootloader..."
grub-install /dev/sda || error_exit "Bootloader installation failed"
update-grub || error_exit "Failed to update grub config"

# === 9. Enable security-related services ===
log_info "Enabling security services..."
for svc in firewalld suricata clamav-daemon maldetect; do
    systemctl enable "$svc" || log_warn "Failed to enable $svc"
done

# === 10. Configure firewall ===
log_info "Configuring firewalld..."
firewall-cmd --set-default-zone=public
firewall-cmd --permanent --add-service=ssh
firewall-cmd --reload || log_warn "Failed to reload firewalld"

# === 11. Enable and start custom services ===
log_info "Setting up NexisOS system services..."
systemctl enable nexis-service || error_exit "Failed to enable nexis-service"
systemctl start nexis-service || log_warn "Failed to start nexis-service"

# === 12. Finalize installation (e.g., DB setup, indexing, etc.) ===
log_info "Finalizing installation..."
nexis-setup --finalize || log_warn "Final setup failed"

# === 13. Cleanup temporary files ===
log_info "Cleaning temporary files..."
rm -rf /tmp/nexis-install || log_warn "Failed to remove /tmp/nexis-install"

# === 14. Log success and remove script ===
log_info "Post-install completed successfully!"
echo "Installation completed successfully on $(date)" >> /var/log/nexis-install.log

log_info "Cleaning up post-install script..."
rm -f /post_install.sh

# === 15. Notify user ===
echo "✅ NexisOS installation is complete. You may now reboot your system."
