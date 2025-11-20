# NexisOS
Main repository for the NexisOS Linux distribution, containing core build
infrastructure, decentralized declarative package manager source, configuration
examples, and tooling.

NexisOS is designed for high-security, reproducible, immutable, system-wide
configuration management similar to NixOS, but built around TOML-based
configuration, decentralized packaging, and modern security enforcement.

Optimized for fast package store operations, atomic rollbacks, sandboxed
builds, SELinux-enforced immutability, TPM-backed binary verification, and
high-performance garbage collection and deduplication.

## Features
A declarative decentralised system level package manager

NexisOS provides a system-level declarative package manager (nexispm) inspired by Nix, but:

- Uses TOML instead of Nix expressions
- Allows any user to easily package software using a minimal, predictable schema
- Is decentralized — packages can be sourced directly from Git, remote archives, registries, or local files without a centralized maintainer or monolithic repository
- Ensures full reproducibility with hashed store paths, deterministic builds, and lockfiles
- All system files are generated & linked from an immutable content-addressed store

---

## 🚀 Download ISO
You can try the latest ISO build of NexisOS by downloading it from SourceForge:

👉 [Download NexisOS ISO](https://sourceforge.net/projects/nexisos/files/latest/download)

> ⚠️ *Note: The ISO is currently experimental and intended for testing and feedback. Expect rapid iteration and updates.*

---

## 🔧 Building From Source Prerequisites

<details>
<summary>Click to see</summary>

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

---

## 🛠️ Build ISO Targets & Testing

<details>
<summary>Click to see</summary>

### Recommended Workflow
```sh
git submodule update --init --recursive # initialize buildroot submodule
make menu                               # recommended TUI
```

### Manual Make Commands
```sh
# initialize buildroot submodule
git submodule update --init --recursive 

make              # Builds x86_64 by default
make ARCH=aarch64 # Builds using nexisos_aarch64_defconfig
make ARCH=riscv64 # Builds using nexisos_riscv64_defconfig

# QEMU test ISO's
make run-qemu              # defaults to ARCH=x86_64
make run-qemu ARCH=x86     # specify arch explicitly
make run-qemu ARCH=aarch64
```

</details>

---

## Installer ISO Script

<details>
<summary>Click to see</summary>

Before the end-user installs distro UEFI or boot settings must be set 

Once precondtions are met the iso launches the whiptail installer TUI
After setting the blank the hardware.toml and blank are created 

</details>


---

## ⚙️ Package Store Design

<details>
<summary>Click to see</summary>

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
<summary>Click to see</summary>

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
<summary>Click to see</summary>

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
<summary>Click to see</summary>

- `nexis generate-hardware` → Regenerate `hardware.toml`
- `nexis resolve-versions` → Update `nexis.lock` with latest versions
- `nexis build` → Build system from config
- `nexis switch` → Switch to new generation
- `nexis rollback` → Rollback to previous generation

</details>

---

## Security Architecture

<details>
<summary>Click to see</summary>


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

```txt
Universal Security Scanner
├── 1. Input Layer
│       ├── Source Code (multi-language)
│       ├── Binaries / compiled artifacts
│       ├── Packages (tarballs, wheels, crates, jars, rpms, debs)
│       └── Metadata (manifests, lockfiles, SBOMs)
│
├── 2. File Normalization Layer
│       ├── Archive Extractor
│       ├── Binary Introspector (ELF/PE/Mach-O)
│       ├── Decompiler / Disassembler
│       ├── SBOM Generator (CycloneDX/SPDX)
│       └── Language Detector & Routing
│
├── 3. Multi-Mode Analysis Engine
│       │
│       ├── 3.1 Static Code Analysis (SAST)
│       │       ├── AST Parsers per language
│       │       ├── Semantic analysis
│       │       ├── Security rule engine (Semgrep/CodeQL-like)
│       │       ├── Dataflow & taint analysis
│       │       ├── Unsafe patterns & zero-day indicators
│       │       └── Secret detection & crypto misuse checks
│       │
│       ├── 3.2 Binary Static Analysis
│       │       ├── Disassembly (Capstone / LLVM / Ghidra)
│       │       ├── Decompilation (RetDec-like)
│       │       ├── Symbol & string extraction
│       │       ├── Malware signatures (YARA)
│       │       ├── Heuristic/entropy anomaly scans
│       │       └── Behavioral pattern inference
│       │
│       ├── 3.3 Dynamic Analysis (optional sandboxing)
│       │       ├── Behavior sandbox for binaries
│       │       ├── Syscall tracing (seccomp, ptrace)
│       │       ├── Network behavior tracing
│       │       ├── Resource abuse heuristics (crypto mining, botnets)
│       │       └── ML-based behavior anomaly detection
│       │
│       └── 3.4 ML / Zero-Day Detection Module
│               ├── Code embeddings (AST/CFG features)
│               ├── Binary embeddings (opcode-level)
│               ├── Outlier detection vs trusted models
│               ├── Malicious pattern classifier
│               ├── Supply-chain anomaly detection (rare patterns, strange publish times)
│               └── Behavioral anomaly ML model
│
├── 4. Dependency & Supply Chain Analysis
│       ├── Direct & transitive dependency graph
│       ├── Integrity & provenance checks
│       ├── Package origin validation (registry → Git → hash)
│       ├── CVE & zero-day heuristic scanner (OSV/RustSec/NVD)
│       ├── Artifact signature verification (SigStore, GPG)
│       ├── Dependency risk scoring (download count, maintainer trust, typosquatting)
│       └── Malicious dependency pattern detection
│
├── 5. Threat Intelligence Layer
│       ├── Known vulnerabilities (OSV.dev, RustSec, NVD)
│       ├── YARA rules
│       ├── Reputation feeds (optional)
│       ├── Hash databases (goodware/badware ML)
│       └── Behavior & anomaly corpora
│
├── 6. Correlation Engine
│       ├── Combine static + binary + dynamic + supply chain signals
│       ├── Risk scoring algorithm
│       ├── Zero-day likelihood estimation
│       └── Confidence models
│
├── 7. Reporting & Output Layer
│       ├── CLI / JSON / HTML report
│       ├── SBOM with vulnerability annotations
│       ├── Behavior trace logs
│       ├── Dependency risk report
│       └── Recommended remediation
│
└── 8. Plugin & Extensibility Framework
        ├── Add new language analyzers
        ├── Add custom detection rules
        ├── Integrate with CI/CD
        ├── Support custom corpora (enterprise)
        └── Custom ML models
```

- Secure Boot verifies bootloader and kernel signatures  
- NexisOS binds system identity to a **TPM attestation identity key**  
- The TPM verifies **integrity of loaded binaries** and prevents unsigned or tampered binaries from executing

### Immutable Core OS
- SELinux enforces that only the package manager (`nexispm`) can modify:
  - `/nexis-store`
  - `/usr`
  - `/etc`
  - `/boot`
  - `$HOME/.local`  
- Even root cannot mutate system files or replace installed software without going through the declarative manager  

### Runtime Security Monitoring
- **Tetragon**: kernel-aware runtime process monitoring for syscall enforcement, privilege escalation prevention, and unexpected behavior detection  
- **Suricata**: inline network IDS/IPS for traffic inspection, C2 detection, and exploitation alerts  

### Advanced Multi-Layer Security Scanner
The scanner integrates into `nexispm` and provides:

    1. Signature-Based Malware Detection
      - Binary signatures, hash-based scanning, YARA rules  
      - Archives, installers, and opaque binaries scanned before installation  
    
    2. Zero-Day & Vulnerability Detection
      - AST and semantic analysis for unknown threats  
      - Detects unsafe APIs, RCE vectors, memory-unsafe code, and obfuscated binaries  
    
    3. Multi-Language SAST
      - Static analysis for multiple languages  
      - Taint and dataflow analysis  
      - Vulnerability rule engine for package code  
    
    4. Binary & Compiled Package Scanning
      - ELF, Mach-O, PE, WASM, and other formats  
      - Disassembly + control flow analysis  
      - Entropy, packer detection, ML-assisted anomaly detection  
    
    5. Supply Chain & Dependency Security
      - Direct and transitive dependency scanning  
      - OSV/RustSec/NVD vulnerability lookups  
      - Typosquatting and dependency confusion detection  
      - Hash-based reproducibility and SBOM generation  
    
    6. ML-Based Anomaly Detection
      - Detects zero-day behavior, malicious build scripts, and unusual
        dependency patterns

NexisOS treats security scanning as part of the package lifecycle — before installation, after build, and during updates.

</details>

---

## ⚙️ Example TOML Configurations

<details>
<summary>Click to see</summary>

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

### Example init service in `nexis_init.toml`
```toml
[[packages]]
name = ""

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

### Declarative File Management
Like Nix’s `writeText` or `environment.etc`, NexisPM allows declarative
creation and tracking of files (configs, dotfiles, system files). Files are
stored in `/nexis-store` with hash-based paths and symlinked into place,
ensuring immutability and reproducibility.

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
fleet = "?"
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
<summary>Click to see</summary>

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

</details>
