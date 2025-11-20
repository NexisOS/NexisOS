distroConfigs/packages/nexispm/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── benches/
│   └── store_operations.rs          # Performance benchmarks
├── tests/
│   ├── integration/
│   │   ├── mod.rs
│   │   ├── build_system.rs          # End-to-end build tests
│   │   ├── rollback.rs              # Generation rollback tests
│   │   ├── fleet_management.rs      # Profile/machine composition tests
│   │   └── gc.rs                    # Garbage collection tests
│   └── fixtures/
│       ├── configs/
│       │   ├── simple.toml
│       │   ├── with-profiles.toml
│       │   └── fleet.toml
│       └── packages/
│           └── test-package/
└── src/
    ├── main.rs                      # CLI entry point
    ├── lib.rs                       # Library exports
    │
    ├── cli/
    │   ├── mod.rs                   # CLI module exports
    │   ├── commands/
    │   │   ├── mod.rs
    │   │   ├── build.rs             # nexis build
    │   │   ├── switch.rs            # nexis switch
    │   │   ├── rollback.rs          # nexis rollback
    │   │   ├── gc.rs                # nexis gc
    │   │   ├── generations.rs       # nexis generations
    │   │   ├── list.rs              # nexis list-{packages,users,machines,profiles}
    │   │   ├── show.rs              # nexis show-{user,machine,profile,deps}
    │   │   ├── verify.rs            # nexis verify
    │   │   ├── query.rs             # nexis query
    │   │   └── deploy.rs            # nexis deploy (fleet management)
    │   └── args.rs                  # CLI argument parsing
    │
    ├── config/
    │   ├── mod.rs                   # Config module exports
    │   ├── types.rs                 # Config structs (System, Package, User, etc.)
    │   ├── loader.rs                # Load and parse TOML files
    │   ├── composer.rs              # Compose base + profiles + machine configs
    │   ├── validator.rs             # Validate configuration
    │   ├── lockfile.rs              # nexis.lock handling
    │   └── schema.rs                # TOML schema definitions
    │
    ├── store/
    │   ├── mod.rs                   # Store module exports
    │   ├── database.rs              # redb wrapper for metadata
    │   ├── layout.rs                # Store directory layout (/nexis-store/ab/cd/...)
    │   ├── objects.rs               # Package/file object storage
    │   ├── reflink.rs               # XFS reflink operations
    │   ├── hash.rs                  # BLAKE3 hashing utilities
    │   ├── gc.rs                    # Garbage collection logic
    │   └── query.rs                 # Query store metadata
    │
    ├── packages/
    │   ├── mod.rs                   # Package module exports
    │   ├── resolver.rs              # Dependency resolution (petgraph)
    │   ├── fetcher.rs               # Download packages (reqwest)
    │   ├── builder.rs               # Build packages (parallel with rayon)
    │   ├── installer.rs             # Install to store and create symlinks
    │   ├── version.rs               # Version resolution (gix, semver)
    │   └── cache.rs                 # Build and download caching
    │
    ├── files/
    │   ├── mod.rs                   # File management module exports
    │   ├── content_address.rs       # Content-addressed file storage
    │   ├── installer.rs             # Install files from config to store
    │   ├── symlink.rs               # Create and manage symlinks
    │   └── permissions.rs           # Handle mode, owner, group
    │
    ├── generations/
    │   ├── mod.rs                   # Generation module exports
    │   ├── manager.rs               # Create, switch, list generations
    │   ├── snapshot.rs              # Snapshot current system state
    │   ├── rollback.rs              # Rollback to previous generation
    │   └── grub.rs                  # GRUB integration for boot-time selection
    │
    ├── users/
    │   ├── mod.rs                   # User management module exports
    │   ├── manager.rs               # Create, configure users
    │   ├── profiles.rs              # Load and apply user profiles
    │   └── home.rs                  # Manage user home directories
    │
    ├── fleet/
    │   ├── mod.rs                   # Fleet management module exports
    │   ├── profiles.rs              # Load profile templates
    │   ├── machines.rs              # Load machine-specific configs
    │   ├── composer.rs              # Compose fleet configurations
    │   └── deployment.rs            # Deploy to multiple machines
    │
    ├── security/
    │   ├── mod.rs                   # Security module exports
    │   ├── selinux.rs               # SELinux policy enforcement
    │   ├── immutability.rs          # Enforce immutable paths
    │   └── permissions.rs           # Capability management
    │
    ├── services/
    │   ├── mod.rs                   # Service management module exports
    │   ├── dinit.rs                 # Dinit service generation
    │   ├── generator.rs             # Generate service files from config
    │   └── manager.rs               # Enable/disable services
    │
    ├── build/
    │   ├── mod.rs                   # Build system module exports
    │   ├── executor.rs              # Execute build commands
    │   ├── environment.rs           # Setup build environment
    │   ├── parallel.rs              # Parallel build orchestration
    │   └── sandbox.rs               # Build sandboxing (future)
    │
    ├── vcs/
    │   ├── mod.rs                   # VCS module exports
    │   ├── git.rs                   # Git operations (gix)
    │   ├── tags.rs                  # Tag parsing and resolution
    │   └── branches.rs              # Branch operations
    │
    ├── utils/
    │   ├── mod.rs                   # Utilities module exports
    │   ├── fs.rs                    # Filesystem utilities
    │   ├── progress.rs              # Progress bars (indicatif)
    │   ├── logging.rs               # Logging setup (tracing)
    │   ├── errors.rs                # Error types and conversions
    │   └── paths.rs                 # Path manipulation helpers
    │
    └── constants.rs                 # Global constants (store paths, etc.)
