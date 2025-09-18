# NexisOS
Main repository for the NexisOS Linux distribution, containing core build infrastructure, package manager source, configuration examples, and tooling.
Optimized for fast package store operations, generation rollbacks, and high-performance GC.

---

## 🔽 Download ISO

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

To build the ISO using one of the provided Buildroot defconfig files:
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

## ⚙️ Possible toml config

<details>
<summary>Click to see possible TOML config example</summary>

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

[system.selinux]
new settings = ?

[system.firewall]
# Choose one firewall backend: "nftables", "iptables", or "firewalld"
# You can switch between them as needed.
backend = "nftables"

[users.root]
password_hash = "$argon2id$v=19$m=65536,t=3,p=4$SOME_BASE64_SALT$SOME_BASE64_HASH"
authorized_keys = [
  "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICWJv... user@example.com",
  "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQC... user2@example.com"
]

[system.locale]
lang = "en_US.UTF-8"
keyboard_layout = "us"

[network]
interface = "eth0"
dhcp = true
# static_ip = "192.168.1.100/24"
# gateway = "192.168.1.1"
# dns = ["8.8.8.8", "8.8.4.4"]

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
patches = ["patches/fix-utf8-bug.patch"]
pre_build_script = "./scripts/setup-env.sh"
post_build_script = "./scripts/custom-cleanup.sh"
build_system = "make"
build_flags = ["-j4"]
context_file = "contexts/vim.cil"
env = { "TERM" = "xterm-256color", "VIMRUNTIME" = "/usr/share/vim/vimfiles" }
runtime_dirs = ["/var/log/vim", "$XDG_RUNTIME_DIR/vim"]

[[packages]]
name = "libpng"
version = "1.6.40"
source = "https://download.sourceforge.net/libpng/libpng-1.6.40.tar.gz"
hash = "sha256:abc123..."
build_system = "configure"
build_flags = ["--enable-static"]
dependencies = ["zlib"]

[config_files.suricata]
path = "/etc/suricata/suricata.yaml"
source = "templates/suricata.yaml.tpl"
owner = "root"
group = "root"
mode = "0640"
variables = { rule_path = "/var/lib/suricata/rules", detect_threads = 4 }

[config_files.ansible]
path = "/etc/ansible/ansible.cfg"
source = "templates/ansible.cfg.tpl"
owner = "root"
group = "root"
mode = "0644"
variables = { inventory = "/etc/ansible/hosts" }

[config_files.clamav]
path = "/etc/clamav/clamd.conf"
source = "templates/clamd.conf.tpl"
owner = "clamav"
group = "clamav"
mode = "0640"
variables = { database_dir = "/var/lib/clamav" }

[dinit_services.network]
name = "network"
type = "scripted"
command = "/etc/dinit.d/network.sh"
depends = []
start_timeout = 20

[dinit_services.sshd]
name = "sshd"
type = "process"
command = "/usr/sbin/sshd"
depends = ["network"]
working_directory = "/"
log_file = "/var/log/sshd.log"
restart = "true"

[nexis.dinit]  # Native dinit service support
# User dinit services
"my-app" = {
    type = "process",
    command = "/home/myuser/.local/bin/my-app",
    depends = ["network"],
    user = "myuser",
    working_directory = "/home/myuser",
    enable = true
}

[[log_rotation]]
path = "/var/log/sshd.log"
max_size_mb = 100
max_files = 7
compress = true
rotate_interval_days = 1

[[log_rotation]]
path = "/var/log/vim"
max_size_mb = 50
max_files = 5
compress = true

# File with custom content
".gitconfig" = { 
    content = '''
[user]
    name = "My Name"
    email = "me@example.com"
[core]
    editor = vim
''' 

# Symlink
".config/nvim" = { 
    symlink = "/etc/nvim-config",
    force = true  # Overwrite if exists
}

# Environment variables
[nexis.environment]
EDITOR = "vim"
BROWSER = "firefox"
PATH = "$PATH:/home/myuser/.local/bin"

# Generation management
[nexis.generations]
keep_last = 10
auto_cleanup = true
```

</details>

## ⚙️ Example SELinux Module Structure

> **Note:**  
> This SELinux policy module is managed by the system and **must not be edited manually**.  
> Please make changes only via the package manager or official policy tools to maintain system integrity and security.

<details>
<summary>Click to see possible SELinux policy example</summary>

```text
policy_module(immutable_paths, 1.0)

# Define read-only types for critical dirs
type immutable_dir_t;
files_read_only(immutable_dir_t)

# Assign context to paths
files_type(immutable_dir_t, "/etc(/.*)?")
files_type(immutable_dir_t, "/usr(/.*)?")
files_type(immutable_dir_t, "/boot(/.*)?")

# Disallow writes to immutable_dir_t by normal users and processes
allow user_t immutable_dir_t:dir { getattr search open };
allow user_t immutable_dir_t:file { getattr open read };
# Deny write, create, unlink permissions explicitly
```

</details>


## ⚙️ Package Store Design

<details>
<summary>Click to see Goals</summary>

```text
Core Goals:

- **Desktop & Server (XFS + RocksDB)**
  - Filesystem: XFS with reflink=1 enabled for both desktop and server users  
  - Store: XFS with reflink-on-ingest (cheap deduplication)  
  - Hashing: BLAKE3 (fast parallel checksums)  
  - GC: staged deletes with Bloom filter acceleration (parallel workers)  
  - Metadata DB: RocksDB with WAL (handles large-scale, high-concurrency workloads)  

- **Desktop Only**
  - Uses ext4 on specific directories to optimize gaming performance:
  - /home (user home directories storing game data, saves, and configs)
  - /opt (optional, for commercial game installations)
  - Custom mount points for games such as /mnt/games or /data/games if configured
  - These ext4 mounts provide mature journaling, low latency for small file operations, and excellent compatibility with game launchers.

- **Common**
  - ACID transactions ensure data consistency
  - Refcounting provides precise garbage collection  
  - Backup of user home files handled externally (rsync/snapshots) – no need to back up entire OS image  
```

</details>

## ⚙️Optimized Store Structure (Bucketed Hashed Store)

<details>
<summary>Click to see example tree</summary>

The store uses a **bucketed hashed directory layout** for fast lookups, deletions, and garbage collection.

```text
/store/
└── ab/
    └── cd/
        ├── abcd1234-vim/
        └── abcd5678-libpng/
```

</details>


## ⚙️Deduplication & Garbage Collection

<details>
<summary>Click to see features</summary>

```text
Deduplication:
- Content is hashed with BLAKE3 on write, ensuring identical files are stored only once.
- Filesystems with reflink support (XFS with reflink=1) enable cheap copy-on-write clones.
- No global store-wide sweep is necessary for cleanup, avoiding expensive full scans.

Garbage Collection:
- A high-performance database tracks reference counts for every stored path.
- GC workflow:
  1. Mark live roots (active generations, pinned profiles).
  2. Decrement refcounts of unreachable items.
  3. Move zero-refcount entries to `/store/.trash/` for safe deletion.
  4. Parallel background workers delete trash contents asynchronously.
- Advanced optimizations include:
  - Bucketed hashed directories for parallel GC without contention.
  - Parallel workers to utilize multi-core systems effectively.
  - Optional Linux io_uring batching to accelerate disk IO.
  - Bloom filters to minimize false-positive checks during reachability analysis.
```

</details>

## ⚙️Rollbacks & Generation Management

<details>
<summary>Click to see features</summary>

```text
NixOS-like features:
- System generations stored as profiles.
- Each generation is a complete system specification.
- GRUB menu entries auto-generated for available generations.
- Atomic upgrades and rollbacks via symlink switching.

Generation Aging & Pruning:
- Older generations can accumulate, increasing storage and metadata overhead.
- Configurable retention policies automatically prune aged or unused generations.
- Generations marked for deletion have their references removed, triggering garbage collection.
- Efficient aging strategy:
  - Keep a configurable number of recent generations (e.g., last 10).
  - Optionally keep pinned or manually marked generations indefinitely.
  - Automatic cleanup runs as a background task to avoid impacting performance.
  - Parallel deletion of generation data using the bucketed hashed store structure.
- Aging policies maintain store performance by minimizing obsolete data.

Performance Highlights:
- **Store / Garbage Collection Cleanup:**
  - NixOS relies heavily on hard links and must recursively traverse the entire store tree to find unreferenced paths.
  - NexisOS avoids this by using a RocksDB-backed refcount database and a bucketed hashed directory structure.
    - Only unreferenced items are touched.
    - Parallel deletion of store entries across hashed buckets.
    - Optional io_uring batching improves disk throughput.
    - Bloom filters reduce unnecessary disk access during reachability analysis.
  - Should be significantly faster and more efficient on XFS with reflink support.

- **Database Performance:**
  - RocksDB is used exclusively for all metadata operations.
  - WAL support ensures durability.
  - Bloom filters accelerate lookups and GC marking.
  - Optimized for large-scale, high-concurrency environments.
```

</details>
