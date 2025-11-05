# NexisOS

A declarative, immutable Linux distribution with atomic rollbacks and content-addressed storage. Built with Buildroot and powered by a custom package manager written in Rust.

---

## 🚀 Quick Start

### Download ISO
Try the latest experimental build:

👉 [Download NexisOS ISO](https://sourceforge.net/projects/nexisos/files/latest/download)

### Build From Source
```bash
git submodule update --init --recursive
make                    # Build x86_64 (default)
make ARCH=aarch64      # Build ARM64
make ARCH=riscv64      # Build RISC-V
```

### Test in QEMU
```bash
make run-qemu              # Test x86_64
make run-qemu ARCH=aarch64 # Test ARM64
```

---

## ✨ Key Features

### 🔒 Immutable System
- Critical paths (`/nexis-store`, `/etc`, `/usr`, `/boot`) are SELinux-enforced immutable
- Only the package manager can modify system state
- User data directories (`/home`, `/var`, `/opt`) remain fully mutable

### 📦 Content-Addressed Storage
- All packages and files stored in `/nexis-store` with hash-based deduplication
- XFS filesystem with reflinks for zero-copy deduplication (no separate optimize command needed)
- Files declared in TOML are content-addressed and symlinked to their destinations

### ⚡ Fast & Parallel
- Parallel package builds with Rayon
- Concurrent metadata operations with DashMap
- BLAKE3 hashing with SIMD acceleration
- Embedded redb database for instant metadata queries

### 🔄 Atomic Rollbacks
- Each system change creates a new generation
- Switch generations instantly
- Rollback any change with zero downtime
- GRUB integration for boot-time generation selection

### 👥 Centralized Administration
- Admin user manages all system and user configurations in one place
- All users, packages, and files declared in TOML
- No per-user imperative installs - everything is tracked
- Single source of truth for entire system state

### 🛡️ Security First
- SELinux mandatory access control
- Sandboxed builds (coming soon)
- Immutability prevents unauthorized system changes
- Version pinning via lock files

---

## 📁 Filesystem Layout

```
/
├── nexis-store/        [XFS with reflinks] Immutable package store
│   ├── packages/       Hash-addressed packages
│   ├── files/          Content-addressed config files
│   └── metadata.redb   Package metadata database
│
├── etc/                [XFS] Immutable system configs (managed)
├── usr/                [XFS] Immutable system binaries (managed)
├── boot/               [ext4] Bootloader and kernels
│
├── home/               [ext4] User data (mutable)
│   └── user/
│       ├── .local/     [Immutable] Managed by package manager
│       ├── Documents/  [Mutable] User files
│       ├── Downloads/  [Mutable] User files
│       └── Games/      [Mutable] User files
│
├── var/                [ext4] Logs and system state (mutable)
├── tmp/                [ext4] Temporary files (mutable)
└── opt/                [ext4] Third-party software (mutable)
```

**Filesystem Strategy:**
- **XFS for `/nexis-store`**: Instant deduplication via reflinks
- **ext4 for user directories**: Optimized for desktop/gaming workloads
- **Symlinks bridge filesystems**: Store content lives on XFS, linked everywhere

---

## ⚙️ Configuration

### Declarative System Config

Everything is declared in TOML files. No imperative package installations.

#### `/etc/nexis/system.toml`
```toml
[system]
hostname = "workstation-01"
timezone = "America/New_York"
version = "0.1.0"

[admin]
user = "bob"

# System packages (available to all users)
[[packages]]
name = "firefox"
version = "latest"
source = "https://github.com/mozilla/firefox.git"

[[packages]]
name = "linux"
version = "~6.10.0"
source = "https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git"

# System files
[[files]]
path = "/etc/motd"
content = "Welcome to NexisOS — Declarative and Secure!"
mode = "0644"
owner = "root"
group = "root"

[[files]]
path = "/etc/ssh/sshd_config"
source = "configs/sshd_config"
mode = "0600"
owner = "root"
group = "root"

# User declarations
[[users]]
name = "bob"
shell = "/bin/bash"
groups = ["wheel", "docker"]

[[users.files]]
path = "/home/bob/.bashrc"
content = '''
export EDITOR=vim
export PATH=$PATH:/nexis-store/bin
alias gs="git status"
'''
mode = "0644"

[[users]]
name = "family-member"
shell = "/bin/bash"
groups = ["users"]

[[users.files]]
path = "/home/family-member/.bashrc"
content = '''
alias games='cd ~/Games'
'''
mode = "0644"
```

---

## 🔧 Package Manager Commands

### System Operations
```bash
# Build and switch to new generation
sudo nexis build

# Apply changes without switching
sudo nexis build --no-switch

# Dry run (show what would change)
sudo nexis build --dry-run

# Switch to a specific generation
sudo nexis switch --generation 42

# Rollback to previous generation
sudo nexis rollback

# List all generations
nexis generations

# Garbage collect unused packages
sudo nexis gc
```

### User Management
```bash
# Add new user (declared in system.toml)
sudo nexis build

# List users
nexis list-users

# Show user's configuration
nexis show-user alice

# List installed packages
nexis list-packages

# Show user's files
nexis list-files --user alice
```

### Debugging
```bash
# Verbose logging
sudo nexis build --log-level trace

# Verify system integrity
nexis verify

# Show package dependencies
nexis show-deps firefox

# Query store
nexis query --hash abc123def456
```

---

## 🔐 SELinux Policy

NexisOS uses SELinux to enforce immutability. Only `nexispm` can modify protected paths.

### Protected Directories
| Path | Access | Notes |
|------|--------|-------|
| `/nexis-store` | Read-only (users), Write (nexispm) | Package store |
| `/etc` | Read-only (users), Write (nexispm) | System configs |
| `/usr` | Read-only (users), Write (nexispm) | System binaries |
| `/boot` | Read-only (users), Write (nexispm) | Kernels |
| `/home/.local` | Read-only (users), Write (nexispm) | User-local installs |
| `/home/*` | Full user control | User data |
| `/var` | Service write access | Logs, state |
| `/tmp` | Full access | Temporary files |
| `/opt` | User/admin install | Third-party software |

---

## 🎯 Design Philosophy

### Simpler Than NixOS
- **No DSL**: Pure TOML configuration instead of Nix language
- **No derivations**: Straightforward package declarations
- **Transparent**: Easy to understand what's installed and where

### More Efficient
- **XFS reflinks**: Automatic deduplication, no manual optimization
- **redb storage**: Fast embedded database, no separate daemon
- **Parallel everything**: Builds, downloads, and GC operations
- **Content addressing**: Same content = one copy, shared everywhere

### User-Friendly for Admins
- **Central management**: One config file for the entire system
- **Declarative users**: All users defined in system.toml
- **Git-friendly**: All configs are text files, perfect for version control
- **Clear separation**: Managed system vs. user data

---

## 🏗️ Build Prerequisites

<details>
<summary>Click to view dependencies</summary>

**Buildroot:**
- build-essential
- make, git, python3
- wget, unzip, rsync, cpio
- libncurses-dev, libssl-dev
- bc, flex, bison, curl

**Package Manager:**
- Rust toolchain (via rustup)
- QEMU + OVMF (for testing)

</details>

---

## 📂 Project Structure

```
NexisOS/
├── buildroot/                  # Buildroot submodule
├── distroConfigs/
│   ├── configs/                # Buildroot configs
│   ├── kernel-configs/         # Kernel configs
│   ├── overlay/                # Root filesystem overlay
│   ├── packages/
│   │   └── nexispm/            # Package manager source
│   │       ├── Cargo.toml
│   │       └── src/
│   └── scripts/                # Build scripts
├── Makefile                    # Top-level build
└── README.md
```

---

## 🚧 Roadmap

- [x] Declarative package management
- [x] Content-addressed file storage
- [x] XFS reflink deduplication
- [x] SELinux immutability enforcement
- [x] Multi-user profile system
- [x] Generation rollbacks
- [ ] Build sandboxing
- [ ] Remote binary cache
- [ ] Cross-compilation support
- [ ] Profile templates (optional, for fleet management)
- [ ] Web-based configuration UI

---

## 🤝 Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## 📜 License

GPL-3.0-or-later - See [LICENSE](LICENSE) for details.

---

## 🔗 Links

- **Repository**: https://github.com/NexisOS/NexisOS
- **Issues**: https://github.com/NexisOS/NexisOS/issues
- **Downloads**: https://sourceforge.net/projects/nexisos

---

## 💡 Example: Complete System Configuration

```toml
[system]
hostname = "dev-workstation"
timezone = "UTC"

[admin]
user = "bob"

# Base system
[[packages]]
name = "linux"
version = "6.10.1"
source = "https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git"

[[packages]]
name = "firefox"
version = "latest"
source = "https://github.com/mozilla/firefox.git"

[[packages]]
name = "steam"
version = "latest"

# System files
[[files]]
path = "/etc/motd"
content = "NexisOS Development Machine"
mode = "0644"
owner = "root"
group = "root"

# Users
[[users]]
name = "bob"
shell = "/bin/bash"
groups = ["wheel"]

[[users.files]]
path = "/home/bob/.bashrc"
content = '''
export EDITOR=vim
alias ll="ls -la"
'''
mode = "0644"

[[users]]
name = "alice"
shell = "/bin/bash"

[[users.files]]
path = "/home/alice/.bashrc"
content = '''
alias games="cd ~/Games"
'''
mode = "0644"
```

**Result**: Reproducible system state, stored in Git, deployed across multiple machines.


