# NexisOS Pull Request Template

## Summary
- **What’s the goal of this PR?** (e.g. bug fix, feature, refactor, packaging change)
- **Which area(s) does it affect?** (e.g. Makefile/Build orchestration, Rust package manager, Buildroot submodule)

## Related Issues
- Closes #___ (if applicable)

---

## Description of Changes
Please summarize your changes. Optionally reference relevant commit messages or issue numbers.

### Build / Makefile
- What commands are affected or added?
- Any specific configurations or ARCH targets tested?

### Rust / nexis‑pkg‑mgr
- New dependencies?
- Changes in key Rust modules (`store`, `gen`, `gc`, etc.)?
- Any new features, bugfixes, or API changes?

### Buildroot
- Submodule updates or new packages?
- Buildroot-related tweaks or defconfig modifications?

### Integration / Orchestration
- How does this interact with the overall build flow?
- Did you update `Makefile`, scripts, or automation?

---

## Checklist
Make sure to check all that apply.

- [ ] I have updated the relevant section(s): `Makefile`, Rust code, Buildroot packages, or scripts
- [ ] I have run:
  - `make` (default or ARCH variant)
  - Rust tests (if applicable)
  - ISO build via Buildroot submodule
- [ ] I have run QEMU with the affected architecture (e.g., `make run-qemu ARCH=x86_64`)
- [ ] I have ensured the PR doesn’t break CI, artifacts, or previously working flows
- [ ] I have updated documentation or `CHANGELOG.md`, if applicable
- [ ] I have added tests or examples demonstrating my change
- [ ] The commit history is clean and meaningful

---

## Testing
Explain how you tested your changes, especially across the stack:

- Makefile / build orchestration: what target?
- Rust: unit tests or manual checks?
- Buildroot: ISO generated? Architecture(s) tested with QEMU?
- Any failure modes you checked?

---

## Screenshots / Logs / Diffs
Optional — for example, build output, QEMU logs, or diffs to defconfigs.

---

## Additional Context
Anything else the reviewers should know.

---

### Example Sections in Use:

```markdown
### Build / Makefile
- Added `clean-all` target to purge Buildroot caches and images.

### Rust / nexis-pkg-mgr
- Fixed bug in `store::ingest` where duplicate dedup logic had off-by-one error.

### Buildroot
- Updated `nexis-pkg.mk` to support cross-compilation for ARM64 (aarch64).

### Testing
- `make ARCH=aarch64` → ISO builds cleanly.
- Ran `make run-qemu ARCH=aarch64` — passed initial boot and shell.
- Added unit test for the new ingest edge case.
