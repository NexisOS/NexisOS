# Introduction

This document defines the governance model of the NexisOS project. The project
uses a structured, hierarchical model to ensure stability, transparency, and
consistent technical direction.

Roles in descending order of authority:

1. Owners
2. Maintainers
3. Collaborators
4. Contributors

This structure is inspired by the Node.js governance model but adapted to meet
the uniqueneeds of a Linux distribution.

--- 

## Owners

Owners are the highest administrative authority and the final decision-makers
for the project. The Owners group includes the project founder and any
additional individuals appointed by existing Owners.

Owner Responsibilities
Owners have:
- Full administrative access to all project repositories
- Authority over:
  - Technical direction and architecture
  - Governance policies (including this document)
  - Release policy and planning
  - Security policy and critical vulnerabilities
  - Repository hosting and structure
  - Conflict resolution
- Authority to appoint or remove Maintainers, Collaborators, and new Owners
- Responsibility to safeguard the long-term health and integrity of the distribution

### Authority and Final Decision-Making

Owners hold final authority when consensus cannot be reached elsewhere.
This includes final decisions on:
- System-wide changes
- Governance updates
- Roadmaps and long-term strategies
- Inclusion or removal of major components
- Any unresolved disputes escalated from Maintainers

Owners may override decisions at lower levels when necessary for security,
legal compliance, or project stability.

---

## Maintainers

Maintainers are trusted project members who oversee specific components,
repositories, or subsystems. They ensure that changes meet standards of
quality, security, and consistency.

### Maintainer Responsibilities

Maintainers have:
- Write/merge access for designated repositories
- Authority to approve or reject PRs
- Responsibility for reviewing and ensuring the quality of contributions
- Participation in technical discussions
- Responsibility for guiding Collaborators and Contributors
- Management of issues, labels, milestones, and workflow processes

Maintainers act as stewards of the components under their care.

### Removal or Inactivity

Owners may move inactive Maintainers to emeritus status after 12 months of
inactivity.
Emeritus Maintainers may request reinstatement.

---

## Collaborators

Collaborators have demonstrated commitment and quality contributions. They
receive limited write access and participate in reviewing PRs and triaging
issues.

### Collaborator Responsibilities

Collaborators may:
- Review and comment on PRs
- Merge PRs when authorized by Maintainers
- Assist with issue triage and labeling
- Provide feedback and guidance to new contributors

They do not have the authority or expectation to make major decisions without
Maintainer involvement.

### Nominating New Collaborators

A Maintainer or Owner may nominate a contributor as a Collaborator.

The process:
1. A Maintainer or Owner opens a nomination discussion summarizing the nominee’s
contributions.
2. Collaborators and Maintainers may voice support or concerns.
3. After 7 days, the nomination passes if there are no blocking objections.
4. An Owner or Maintainer performs onboarding.

If consensus cannot be reached, the decision escalates to the Owners.

---

## Contributors

Contributors include anyone who:
- Submits issues
- Submits pull requests
- Helps test, review, or document
- Provides ideas or feedback

Contributors do not need special permissions—they contribute through public
participation.
Many Contributors eventually become Collaborators.

---

## Pull Requests From Non-Contributors

### Who Can Submit PRs
Anyone regardless of role, experience, or involvement—may open a PR.
This includes first-time contributors and external developers.

Open participation is central to the project’s values.

### Review and Approval Requirements
PRs from non-Contributors follow the same rules as all PRs:
- Collaborators may review and comment.
- Maintainers must approve PRs affecting their areas.
- Critical system components (kernel, bootloader, security modules, packaging
  system) may require Owner review.
- All PRs must pass:
  - CI tests
  - Required quality checks
  - Security review (when applicable)

PRs may be closed if they:
- Do not align with project goals
- Introduce risks
- Fail to meet standards
- Do not receive necessary updates from the author

### Encouraging New Contributors

Project members should:
- Be welcoming
- Provide constructive feedback
- Help explain project structure
- Suggest improvements
- Offer guidance for future contributions

This helps grow a healthy community.

### Security Considerations

Because this is a Linux distribution, PRs touching sensitive areas require more
scrutiny:
- Kernel configuration or patches
- init system or bootloader
- Package build scripts
- Cryptographic tooling
- Update mechanisms
- System security policies

Maintainers or Owners may request additional reviews or verification steps for
new contributors working in sensitive areas.

---

## Decision-Making Process

### Consensus Seeking
The project uses a consensus-seeking model:
1. Discussion happens publicly through GitHub issues and PRs.
2. Collaborators and Maintainers work toward agreement.
3. If consensus cannot be reached, a Maintainer may call for escalation.

### Escalation
If an impasse occurs:
1. Maintainers attempt resolution.
2. If unresolved, the issue escalates to the Owners.
3. Owners make the final decision, which is binding.

---

## Meetings

Meetings may be held as needed:
- Maintainer technical meetings
- Owner strategic meetings
- Public community meetings (optional)

Public meetings should record notes unless private or security-sensitive.

---

## Forking and the GPL-3.0 License

The NexisOS project is licensed under the GNU General Public License v3.0
(GPL-3.0).

Under the GPL-3.0 license:
- Anyone may fork the project.
- Any fork must remain licensed under GPL-3.0 or a compatible license.
- Owners cannot restrict lawful forking, as this is a core freedom under GPL.

Owners may take legal action if the GPL-3.0 license terms are violated.

---

## Amending this Document
Amendments require:
1. Approval from at least two Maintainers
2. No Maintainer objections within 7 days
3. Final Owner approval

Owners may veto or modify amendments as needed.
