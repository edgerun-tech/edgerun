# Environment Profiles for Local/Remote Stack Config

All service env files are now rendered from a selected profile and written to
`~/.config/edgerun/*.env`.

- The files in `scripts/systemd/env/profiles/<profile>/` are the **source of truth**.
- `install-user-services.sh` defaults to `local` when no profile is supplied.
- `*.override.env` files can be added locally for secrets or machine-specific one-off values and
  are loaded after profile files.
- Installer validation checks required keys and key formats before restarting services.

Quick profile list and command:

```bash
./scripts/systemd/install-user-services.sh local
./scripts/systemd/install-user-services.sh dev 3
```

Examples for start:

```bash
./scripts/systemd/start-user-stack.sh 3 local
./scripts/systemd/start-user-stack.sh --workers 3 --profile dev
```

Available profile dirs:
- `local`
- `dev`
- `test`
- `main`
