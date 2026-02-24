# CI Required Checks Policy

Branch protection on `main` should require the following check runs:

- `CI / CI Summary`
- `Dependency Review / dependency-review`
- `Pipeline Health / Publish Pipeline Health Summary`

Policy source of truth:
- `.github/required-checks.txt`

Operator steps (GitHub UI):
1. Open repository `Settings` -> `Branches` -> protection rule for `main`.
2. Enable `Require status checks to pass before merging`.
3. Add each required check listed above exactly by name.
4. Enable `Require branches to be up to date before merging`.

Notes:
- `CI / CI Summary` is the aggregate gate over major CI jobs.
- `Pipeline Health` provides cross-workflow visibility and should remain required so missing follow-up workflow signals are visible.
