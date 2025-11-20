# Contributing to NexisOS

We welcome contributions from the community! Whether you're fixing bugs,
suggesting new features, or improving documentation, your input helps make
NexisOS better for everyone.

This document explains how to contribute safely and effectively, in alignment
with our [GOVERNANCE.md](./GOVERNANCE.md).

---

## Roles and Responsibilities

* **Owners**: Final authority on technical and governance decisions, security
  issues, and major disputes.
* **Maintainers**: Review and approve changes to their areas, ensure quality,
  and guide contributors.
* **Collaborators**: Review PRs, assist with issue triage, and may merge
  changes when authorized.
* **Contributors**: Anyone submitting issues, PRs, or other contributions.
  Contributors can become Collaborators over time through sustained
  high-quality contributions.

---

## Reporting Bugs

* **Security issues** should **not** be reported via GitHub issues. Please
  follow our [security policy]() to report them safely.
* Before opening a new issue, check the [existing issues]() to see if it has
  already been reported.
* If it hasn’t, open a new issue with:
  - A clear title and description
  - Steps to reproduce the issue
  - Expected vs. actual behavior
  - System information (e.g., distro version, architecture, logs)

---

## Submitting Fixes

* Fork the repository and create a new branch for your fix.
* Submit a [pull request]() including:
  - A clear explanation of the issue it solves
  - A reference to the related issue (if applicable)
  - Tests or logs demonstrating the fix (if needed)
* Keep commits focused—one change per commit with descriptive commit messages.
* All PRs are reviewed by Collaborators or Maintainers. PRs affecting critical
  components may require Owner review (e.g., kernel, bootloader, package
  management, or security-sensitive code).

---

## Submitting Features or Major Changes

* For new features or major architectural changes:
  - Start a discussion or join our community chat to propose the idea first.
  - Don’t open a GitHub issue for feature proposals—issues are for bugs.
* Once the idea is validated:
  - Submit a pull request with your implementation.
  - Maintainers will review the PR; Owners may review if the change affects
    core system components or security.
* High-impact or security-related changes require additional scrutiny and review.

---

## Pull Request Review Process

* Anyone can submit a PR, including first-time contributors.
* Collaborators and Maintainers will review submissions before merging.
* PRs affecting critical system components may require additional review and
  approval by Owners.
* Feedback may be requested, and changes may be needed before merging.
* Constructive discussion is encouraged; respectful communication is expected.

---

## Code Style & Conventions

To keep the codebase clean and readable:

* Use 2 spaces for indentation, no tabs.
* Keep code modular and readable—avoid deeply nested logic.
* Write comments where logic is not obvious.
* Follow existing naming and file structure patterns.
* Use descriptive commit messages. For example:

```sh
$ git commit -m "Fix bootloader error on UEFI systems
Corrected a misconfiguration in the GRUB install script that caused
boot failures on some UEFI-based setups. Added checks for system
architecture and fallback handling."
```

## Documentation Contributions
- Fixing typos, adding examples, or improving clarity in docs is always welcome.
- Submit a pull request just like code changes.

---

## Progression to Collaborator Status

- Consistently submitting high-quality PRs and engaging constructively in
  reviews may lead to being nominated as a Collaborator.
- Collaborator status provides additional permissions such as merging PRs and
  assisting with issue triage.
- For more details, see GOVERNANCE.md

---

Thanks for helping improve NexisOS! Every contribution strengthens the project
and the community. 🙌
