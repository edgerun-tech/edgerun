# INTENT_UI_PROFILE_BOOTSTRAP_PROFILE_ONLY_LOCAL_FS_V1

## Goal
Move onboarding to profile-only operation:
1. Remove ephemeral/no-profile mode.
2. Keep only `Create profile` and `Load profile` flows.
3. After profile creation, guide users to download encrypted profile backup and optionally connect a local folder using browser File System Access API to save the profile file.

## Non-Goals
- Full cloud sync backend implementation for Google Drive/Git in this change.
- Replacing WebAuthn/YubiKey bootstrap cryptographic flow.

## Security And Constraints
- Fail-closed: protected surfaces remain blocked unless profile is loaded.
- Keep passphrase + WebAuthn verification requirements.
- Never store decrypted profile payload outside active runtime secret context.
- Continue to store encrypted profile blobs only.

## Design
- Remove ephemeral activation path from `profile-runtime` and bootstrap gate.
- Bootstrap gate allows only create/load tabs.
- Create flow:
  - creates encrypted profile,
  - activates profile runtime,
  - presents post-create actions:
    - download encrypted profile file,
    - connect local folder and write encrypted profile file via `showDirectoryPicker`.
  - user continues explicitly to workspace.
- Update UI strings referencing ephemeral mode to profile-only language.

## Acceptance Criteria
- No UI control exists to proceed without profile.
- First-run requires create/load profile completion before entering workspace.
- Create mode shows post-create backup guidance and download action.
- Local-folder save action is available when File System Access API exists.
- Existing integration and profile-gated window behavior remains intact.

## Rollout
- Frontend-only rollout.
- Backward compatible with existing stored profile blobs/session profile mode.

## Rollback
- Revert `ProfileBootstrapGate` and `profile-runtime` changes.
- Restore prior ephemeral session controls.
