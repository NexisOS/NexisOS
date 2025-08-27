# Contributing to Blank

We welcome contributions from the community! Whether you're fixing bugs, suggesting new features, or improving documentation, your input helps make Blank better for everyone.

---

## Reporting Bugs

* **Security issues** should **not** be reported via GitHub issues. Please refer to our [security policy]() for how to report them safely.
* Before opening a new issue, check the [existing issues]() to see if your bug has already been reported.
* If it hasn't, [open a new issue]() with:
  - A clear title and description
  - Steps to reproduce the issue
  - Expected vs. actual behavior
  - System information (e.g., distro version, architecture, logs)

---

## Submitting Fixes

* Fork the repo and create a new branch for your fix.
* Submit a [pull request]() with:
  - A clear explanation of the issue it solves
  - A reference to the related issue (if applicable)
  - Tests or logs demonstrating the fix (if needed)
* Keep your commits focused—one change per commit, with descriptive commit messages.

---

## Submitting Features or Changes

* For new features or major changes:
  - [Start a discussion]() or join our community chat to propose the idea first.
  - Don’t open a GitHub issue for feature proposals—issues are for bugs.
* Once the idea is validated, submit a pull request with your implementation.

---

## Code Style & Conventions

To keep the codebase clean and readable, follow these guidelines:

* Use 2 spaces for indentation, no tabs.
* Keep code modular and readable—avoid deeply nested logic.
* Write comments where the logic isn’t obvious.
* Follow the existing naming and file structure patterns.
* Write descriptive commit messages. For example:

```sh
$ git commit -m "Fix bootloader error on UEFI systems
Corrected a misconfiguration in the GRUB install script that caused
boot failures on some UEFI-based setups. Added checks for system
architecture and fallback handling."
```
---

## Documentation Contributions

* Fixing typos, adding examples, or improving clarity in docs is always welcome.
* Submit a pull request as with code changes.

---

Thanks for helping improve Blank! Every contribution makes the project stronger. 🙌
