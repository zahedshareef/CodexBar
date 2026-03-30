# Branching

Mission branch topology and expectations.

**What belongs here:** local branch names, branch baselines, separation rules.

---

## Mission base branch

- Local mission base branch name: `mission/feb91986-base`
- This branch is expected to contain the mission infrastructure (`.factory/`, mission state references, worker skills) and no milestone implementation work

## Deliverable branches

Workers must keep implementation work separated onto these local branches:

- `mission/pr14-language-toggle`
- `mission/pr15-claude-parser`
- `mission/issue13-msi-auth-recovery`

## Separation rules

- Each deliverable branch must start from `mission/feb91986-base`, not from another milestone branch
- Do not stack milestone implementation commits on top of one another
- A branch-prep feature is responsible for switching the working tree to the correct milestone branch before implementation features run
