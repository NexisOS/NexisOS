# NexisOS
Main repository for the NexisOS Linux distribution, containing core build infrastructure, package manager source, configuration examples, and tooling.

---

## 🔽 Download ISO

You can try the latest ISO build of NexisOS by downloading it from SourceForge:

👉 [Download NexisOS ISO](https://sourceforge.net/projects/nexisos/files/latest/download)

> ⚠️ *Note: The ISO is currently experimental and intended for testing and feedback. Expect rapid iteration and updates.*

---

## 📁 Possible Directory Layout

<details>
<summary>Click to see possible directory structure</summary>

```text
NexisOS/
├── depends/                           # All custom code, tools, and scripts
│   ├── configs/                       # Defconfig files to build NexisOS minimal installer ISO
│   │   ├── NexisOS_x86_64_defconfig
│   │   ├── NexisOS_aarch64_defconfig
│   │   └── NexisOS_riscv64_defconfig
│   │
│   ├── kernel-configs/                # Linux kernel config files per architecture
│   │   ├── linux-x86_64.config
│   │   ├── linux-aarch64.config
│   │   └── linux-riscv64.config
│   │
│   ├── package_manager/               # Rust source for NexisOS package manager
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── cli.rs
│   │       ├── config.rs
│   │       ├── main.rs
│   │       ├── manifest.rs
│   │       ├── packages.rs
│   │       ├── rollback.rs
│   │       ├── store.rs
│   │       ├── types.rs
│   │       └── util.rs
│   │
│   ├── package/                       # Buildroot package definition for nexpm
│   │   ├── Config.in
│   │   └── nexpm/
│   │       ├── Config.in
│   │       └── nexpm.mk               # Build instructions to compile Rust package manager
│   │
│   ├── overlay/                       # Root filesystem overlay for Buildroot
│   │   ├── etc/
│   │   │   ├── motd                   # Message of the day
│   │   │   └── skel/
│   │   │       └── .config/
│   │   │           └── autostart/
│   │   │               └── nexis-welcome.desktop
│   │   │
│   │   └── root/
│   │       ├── scripts/               # Runtime scripts, installer, post-install hooks
│   │       │   ├── install.sh
│   │       │   └── post-install.sh
│   │       └── package_manager/       # Runtime config, data for package manager (no source)
│   │           └── config.toml
│   │
│   └── scripts/                       # Helper/build scripts for project (optional)
│       ├── build_package_manager.sh   # Optional: compile package manager manually
│       ├── install.sh
│       └── post-install.sh
│
├── buildroot/                         # Buildroot submodule (Linux build system)
├── buildroot_backup_imgs/             # Backups of Buildroot output images
├── Makefile                           # Main build orchestrator for NexisOS ISO
├── README.md
├── LICENSE
├── VERSION
├── CHANGELOG.md
├── CONTRIBUTING.md
└── SECURITY.md
```

</details>

## 🔧 Prerequisites

<details>
<summary>Click to see if you have the following Prj dependencies</summary>

```text
Buildroot
├── build-essential
├── make
├── git
├── python3
├── wget
├── unzip
├── rsync
├── cpio
├── libncurses-dev
├── libssl-dev
├── bc
├── flex
├── bison
└── curl

Prj
├── package_manager
│   └── rustup
└── qemu
    └── ovmf # UEFI support
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

After the build completes, the ISO and related images will be located in:
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

[system.selinux]
enabled = true
mode = "enforcing"    # can be "permissive" or "disabled" for testing

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
