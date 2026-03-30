---
name: branch-prep-worker
description: Prepare the working tree for a milestone by switching to the correct local deliverable branch from the mission base branch.
---

# Branch Prep Worker

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the work procedure.

## When to Use This Skill

Use for milestone setup features whose only job is to put the repo on the correct local branch baseline before implementation work begins.

## Required Skills

None.

## Work Procedure

1. Read the feature description carefully and identify:
   - the required target branch name
   - the required base branch (`mission/feb91986-base`)
2. Confirm the working tree is clean before changing branches. If not, return to orchestrator immediately.
3. Verify the mission base branch exists locally.
4. Create the target branch from the mission base branch if it does not exist, or reset/switch it back to the mission base branch if the feature description explicitly requires a fresh branch baseline.
5. Verify the final branch name and HEAD commit.
6. Do not modify product code in this feature.
7. Run read-only git verification commands and record them in the handoff.

## Example Handoff

```json
{
  "salientSummary": "Prepared the repo for the PR #14 milestone by switching to `mission/pr14-language-toggle` from `mission/feb91986-base`. Verified the working tree stayed clean and the branch now points at the mission base commit.",
  "whatWasImplemented": "Created the local branch `mission/pr14-language-toggle` from `mission/feb91986-base`, switched the working tree to it, and verified that no implementation files changed during branch preparation.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      {
        "command": "git status --short",
        "exitCode": 0,
        "observation": "Working tree was clean before and after the branch switch."
      },
      {
        "command": "git branch --show-current",
        "exitCode": 0,
        "observation": "Current branch is `mission/pr14-language-toggle`."
      },
      {
        "command": "git rev-parse --short HEAD",
        "exitCode": 0,
        "observation": "HEAD points at the expected mission-base-derived commit."
      }
    ],
    "interactiveChecks": []
  },
  "tests": {
    "added": []
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- The working tree is not clean before switching branches
- `mission/feb91986-base` does not exist locally
- The feature description is ambiguous about which branch should be created or reset
