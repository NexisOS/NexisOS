# NexisOS
Main repository for the NexisOS Linux distribution, containing core build
infrastructure, package manager source, configuration examples, and tooling.
Optimized for fast package store operations, generation rollbacks, sandboxed
builds, SELinux-based immutability enforcement, and high-performance GC.

## Security
Uses secure boot TPM attestation identity key to verify integraty of loaded
binaries.

Uses tetragon and selinux to prevent root from altering immutable core OS
files. Also uses suricata for network intrusion detection and prevention. 

---

## 🚀 Download ISO
You can try the latest ISO build of NexisOS by downloading it from SourceForge:

👉 [Download NexisOS ISO](https://sourceforge.net/projects/nexisos/files/latest/download)

> ⚠️ *Note: The ISO is currently experimental and intended for testing and feedback. Expect rapid iteration and updates.*

---

## 🔧 Building From Source Prerequisites

<details>
<summary>Click to see dependencies</summary>

```text
Buildroot:
- build-essential
- make
- git
- python3
- wget
- unzip
- rsync
- cpio
- libncurses-dev
- libssl-dev
- bc
- flex
- bison
- curl

Project:
- Rust (via rustup) for package_manager
- QEMU + OVMF (UEFI support)
```

</details>

## 🛠️ Build the NexisOS ISO Targets

<details>
<summary>Click to see how to build iso</summary>

```sh
git submodule update --init --recursive # initialize buildroot submodule
make                                    # Builds x86_64 by default
make ARCH=aarch64                       # Builds using nexisos_aarch64_defconfig
make ARCH=riscv64                       # Builds using nexisos_riscv64_defconfig
```

Output locations:
```sh
buildroot_backup_imgs/x86/output/images/bzImage
buildroot_backup_imgs/x86/output/images/rootfs.ext2
buildroot_backup_imgs/x86/output/images/run-qemu.sh

buildroot_backup_imgs/aarch64/output/images/bzImage
buildroot_backup_imgs/aarch64/output/images/rootfs.ext2
buildroot_backup_imgs/aarch64/output/images/run-qemu.sh

buildroot_backup_imgs/riscv64/output/images/bzImage
buildroot_backup_imgs/riscv64/output/images/rootfs.ext2
buildroot_backup_imgs/riscv64/output/images/run-qemu.sh
```

</details>

## 🖥️ Running NexisOS in QEMU for Testing

<details>
<summary>Click to see how to test distro in virt</summary>

```sh
make run-qemu              # defaults to ARCH=x86_64
make run-qemu ARCH=x86     # specify arch explicitly
make run-qemu ARCH=aarch64
```

</details>

---

## ⚙️ Example TOML Configurations

<details>
<summary>Click to see possible TOML config examples</summary>

### Minimal `config.toml`
```toml
[system]
hostname = "myhost"
timezone = "UTC"
version = "0.1.0"
kernel = "linux-6.9.2"
kernel_source = "https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.9.2.tar.xz"
kernel_config = "configs/kernel-default.config"

[users.myuser]
password_hash = "$argon2id$v=19$m=65536,t=3,p=4$..."
shell = "/bin/bash"
home = "/home/myuser"

[system.locale]
lang = "en_US.UTF-8"
keyboard_layout = "us"

[network]
interface = "eth0"
dhcp = true

[includes]
paths = [
  "packages/hardware.toml",
  "packages/editors.toml",
  "packages/devtools.toml"
]

[[packages]]
name = "vim"
version = "latest"
prebuilt = "https://github.com/vim/vim/releases/download/{tag}/vim-{tag}-linux-{arch}.tar.gz"
fallback_to_source = true
source = "https://github.com/vim/vim.git"
```

### `hardware.toml`
```toml
[cpu]
model = "amd_ryzen"
cores = 16
threads = 32
flags = ["sse4_2", "avx2", "aes"]

[gpu]
model = "nvidia-rtx-4090"
driver = "nvidia"

[storage]
devices = [
  { path = "/dev/nvme0n1", fs = "xfs", mount = "/", reflink = true },
  { path = "/dev/sda1", fs = "ext4", mount = "/home" }
]

[network]
interfaces = [
  { name = "eth0", mac = "00:11:22:33:44:55", dhcp = true }
]
```

### `packages/desktop.toml`
```toml
[[packages]]
name = "firefox"
version = "latest"
source = "https://github.com/mozilla/firefox.git"

[[packages]]
name = "steam"
version = "latest"
provider = "steam" # future version provider extension
```

### `nexis.lock`
```toml
[[packages]]
name = "firefox"
version = "120.0"
resolved = "https://github.com/mozilla/firefox.git?tag=v120.0"

[[packages]]
name = "linux"
version = "6.10.1"
resolved = "https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git?tag=v6.10.1"
```

### Example dinit service in `nginx.toml`
```toml
[[packages]]
name = "nginx"

[packages.dinit_services.nginx]
type = "process"
command = "/usr/sbin/nginx -g 'daemon off;'"
depends = ["network", "filesystem"]
user = "nginx"
working_directory = "/var/www"
restart = "always"
log_file = "/var/log/nginx/access.log"
start_timeout = 30
enable = true
```

### Declarative File Management (`files`)
Like Nix’s `writeText` or `environment.etc`, NexisPM allows declarative creation and tracking of files (configs, dotfiles, system files). Files are stored in `/nexis-store` with hash-based paths and symlinked into place, ensuring immutability and reproducibility.

```toml
[[files]]
path = "/etc/motd"
content = "Welcome to NexisOS — Managed by nexispm"
mode = "0644"
owner = "root"
group = "root"

[[files]]
path = "/home/myuser/.config/fish/config.fish"
content = '''
set -g -x PATH $PATH /nexis-store/bin
alias ll="ls -la"
'''
mode = "0644"
owner = "myuser"
group = "users"

[[files]]
path = "/home/myuser/.local/share/nexispm/test.txt"
source = "files/test.txt"   # reference to repo-tracked file
```

- `path` → target install path
- `content` → inline text (hash stored in `/nexis-store`)
- `source` → import an existing file into the store
- `mode`, `owner`, `group` → permission metadata

This gives one **unified method**: whether inline or external, all files are normalized into the store, then linked to their declared `path`.

### Default `files.toml` Template
A starter template for user and system file management:
```toml
# System Message of the Day
[[files]]
path = "/etc/motd"
content = "Welcome to NexisOS — Declarative and Secure!"
mode = "0644"
owner = "root"
group = "root"

# User shell configuration
[[files]]
path = "/home/user/.bashrc"
content = '''
# Custom aliases
alias ll="ls -la"
export EDITOR=vim
'''
mode = "0644"
owner = "user"
group = "users"

# Dotfile for fish shell
[[files]]
path = "/home/user/.config/fish/config.fish"
content = '''
set -g -x PATH $PATH /nexis-store/bin
alias gs="git status"
'''
mode = "0644"
owner = "user"
group = "users"

# Import external tracked file
[[files]]
path = "/home/user/.config/nvim/init.vim"
source = "dotfiles/init.vim"
mode = "0644"
owner = "user"
group = "users"
```

</details>

---

## ⚙️ SELinux Enforced Immutability

<details>
<summary>Click to see possible SELinux policy example</summary>

NexisOS uses SELinux to enforce immutability on critical directories and the package store. This ensures only the package manager (`nexispm`) has permission to modify these paths, protecting against accidental or malicious tampering.

Key protected directories:
- `/nexis-store` → Package store (immutable except via `nexispm`)
- `/etc` → System configuration
- `/usr` → System binaries and libraries
- `/boot` → Kernel and bootloader files
- `$HOME/.local` → User-level managed installs (immutable except via `nexispm`)

### Example Policy
```text
policy_module(immutable_paths, 1.0)

type immutable_dir_t;
files_read_only(immutable_dir_t)

files_type(immutable_dir_t, "/nexis-store(/.*)?")
files_type(immutable_dir_t, "/etc(/.*)?")
files_type(immutable_dir_t, "/usr(/.*)?")
files_type(immutable_dir_t, "/boot(/.*)?")
files_type(immutable_dir_t, "/home/.local(/.*)?")

type nexispm_t;
allow nexispm_t immutable_dir_t:dir { create write remove_name add_name };
allow nexispm_t immutable_dir_t:file { create write unlink append rename };

allow user_t immutable_dir_t:dir { getattr search open };
allow user_t immutable_dir_t:file { getattr open read };
```

### Directory Access Model (ASCII)
```text
/
├── nexis-store/        [Immutable | managed by nexispm]
├── etc/                [Immutable | managed by nexispm]
├── usr/                [Immutable | managed by nexispm]
├── boot/               [Immutable | managed by nexispm]
├── var/                [Mutable | system services and logs]
├── tmp/                [Mutable | temporary files]
├── home/
│   ├── user/
│   │   ├── .local/     [Immutable | managed by nexispm]
│   │   ├── Documents/  [Mutable | user data]
│   │   ├── Downloads/  [Mutable | user data]
│   │   └── Games/      [Mutable | user data]
└── opt/                [Mutable | optional third-party software]
```

### SELinux Enforcement Matrix
| Directory        | SELinux Type       | Actor          | Access Rights                   | Notes |
|------------------|-------------------|---------------|---------------------------------|-------|
| `/nexis-store`   | `immutable_dir_t` | `nexispm_t`   | read, write, create, delete     | Only package manager can modify store |
| `/etc`           | `immutable_dir_t` | `nexispm_t`   | read, write, create, delete     | System configs enforced immutable |
| `/usr`           | `immutable_dir_t` | `nexispm_t`   | read, write, create, delete     | Binaries & libraries locked |
| `/boot`          | `immutable_dir_t` | `nexispm_t`   | read, write, create, delete     | Kernel and bootloader managed declaratively |
| `/home/.local`   | `immutable_dir_t` | `nexispm_t`   | read, write, create, delete     | User-local installs controlled only by package manager |
| `/var`           | `var_t`           | `system_u:system_r:services_t` | read, write, append             | Service and log storage |
| `/tmp`           | `tmp_t`           | `user_t` / services | read, write, append          | Ephemeral files |
| `/home` (except `.local`) | `home_t` | `user_t`       | full user control               | User documents, data, personal files |
| `/opt`           | `opt_t`           | `user_t` / admins | install third-party software | Safe mutable location outside of store |

This approach balances **safety** (immutability of core paths), **performance** (minimal SELinux checks beyond boundaries), and **maintainability** (clear separation between declarative and user-managed paths). It also prevents **dependency hell** by ensuring all system-managed packages flow through `nexispm` rather than ad-hoc installs.

</details>

---

## ⚙️ Package Store Design

<details>
<summary>Click to see Goals</summary>

Core Goals:
- Filesystem: XFS with reflink=1 for store deduplication
- Store metadata: RocksDB with WAL + Bloom filters
- Hashing: BLAKE3 (parallel)
- GC: refcounted, async deletion, bucketed hashed directory layout
- Desktop performance: ext4 recommended for `/home`, `/var`, `/opt`

Example store layout:
```text
/store/
└── ab/
    └── cd/
        ├── abcd1234-vim/
        └── abcd5678-libpng/
```

</details>

---

## ⚙️ Garbage Collection & Rollbacks

<details>
<summary>Click to see features</summary>

- Refcount DB tracks all store objects
- Unreferenced objects moved to `/store/.trash/` before async delete
- Parallel GC workers with optional io_uring acceleration
- Rollbacks:
  - Generations stored as complete configs
  - Switch generations atomically
  - Auto-generated GRUB entries
  - Keep last N generations (configurable)

</details>

---

## ⚙️ Version Providers


<details>
<summary>Click to see details</summary>

By default, NexisPM resolves versions via Git tags + semver. With the `version-providers` feature, external registries are supported.

Example:
```toml
[[packages]]
name = "numpy"
version = "latest"
provider = "pypi"

[[packages]]
name = "express"
version = "^4.0"
provider = "npm"

[[packages]]
name = "serde"
version = "^1.0"
provider = "cratesio"
```

</details>

---

## ⚙️ Commands


<details>
<summary>Click to see example cli commands</summary>

- `nexis generate-hardware` → Regenerate `hardware.toml`
- `nexis resolve-versions` → Update `nexis.lock` with latest versions
- `nexis build` → Build system from config
- `nexis switch` → Switch to new generation
- `nexis rollback` → Rollback to previous generation

</details>
