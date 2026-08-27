# Triage Labels

The skills speak in terms of five canonical triage roles. In this repo they are written as the `status:` frontmatter value of a spec or issue file under `docs/specs/` (see `issue-tracker.md`). There are no GitHub labels.

| Role in mattpocock/skills | `status:` value    | Meaning                                  |
| ------------------------- | ------------------ | ---------------------------------------- |
| `needs-triage`            | `needs-triage`     | Maintainer needs to evaluate this item   |
| `needs-info`              | `needs-info`       | Waiting on maintainer for more information |
| `ready-for-agent`         | `ready-for-agent`  | Fully specified, ready for an AFK agent  |
| `ready-for-human`         | `ready-for-human`  | Requires human implementation            |
| `wontfix`                 | `wontfix`          | Will not be actioned                     |

Additional lifecycle values used after triage: `in-progress`, `done`.

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), set the `status:` field to the corresponding value.
