# PocketBase Parity Checklist

Source inspected: `git@github.com:pocketbase/pocketbase.git` at commit
`a286d28bf9ee76d5f7900458f713dee8989ed35d` (`2026-05-15 fixed changelog typo`).
Clone path used for extraction: `/tmp/pocketbase-src`.

Status legend:

- `[x]` implemented in current `edgerun-pocketbase` slice
- `[~]` partially implemented, not PocketBase-equivalent
- `[ ]` not implemented

## Binary And Runtime

- [x] Native HTTP server on configurable bind address.
- [~] `serve [domain(s)]` command semantics, HTTP/HTTPS bind flags, domain handling, and autocert cache; current binary supports `--https-bind`, PEM cert/key loading, persistent EdgeRun self-signed autocert generation, and ACME challenge material routes.
- [~] Embedded admin UI under `/_/{path...}` backed by an EdgeRun UI Core scene endpoint; HTML is only a canvas bridge, and the visible admin surface is generated from Rust shadcn/ui-core components. Full PocketBase SPA workflow parity remains pending.
- [ ] UI extension serving under `/_/extensions/...` and `/_/extensions.js`.
- [ ] Public/static file serving extension points.
- [~] `superuser` CLI group: `upsert`, `create`, `update`, `delete`; `otp` and `ips` remain pending.
- [ ] Migration CLI/plugin: create, up, down, history, collections snapshot automigration.
- [ ] GitHub update plugin behavior.
- [~] Minimal QuickJS hook runtime at `/api/edgerun/hooks/run` with the QuickJS executable embedded in the `edgerun-pocketbase` binary; full plugin bindings remain pending.
- [~] ACME HTTP-01/DNS-01/TLS-ALPN-01 challenge material and HTTP-01 well-known serving; full ACME account/order/finalize/certificate issuance loop remains pending.
- [ ] Bootstrap/serve/terminate lifecycle hooks.
- [ ] Cross-process notify directory behavior for multiple instances sharing one `pb_data`.

## Storage And Database

- [~] Durable data directory and local file storage, with EdgeRun Storage derived-db state, encrypted blob mirrors for state snapshots and uploads.
- [~] SQLite replacement path: `data.db` is an `edgerun-storage::DerivedDb` append-only database with typed rows for collections, records, settings, auth-flow tokens, logs, crons, admins, migrations, realtime events, plus a legacy whole-state row; SQLite file/page compatibility and `auxiliary.db` remain pending.
- [~] Transactional record/collection/settings writes through `DerivedDb` diff batches that append only changed/deleted projection rows; record CRUD, collection CRUD/import/truncate, settings, admins, logs, crons, migrations, and auth-flow token issue/consume now use typed row deltas. Whole-state persistence remains for batch and backup restore.
- [ ] Query timeout, retry behavior, max open/idle connection configuration.
- [ ] System migrations table `_migrations`.
- [ ] System migrations from `migrations/*.go`.
- [~] App migration tracking.
- [ ] Atomic backup restore with rollback on failure.
- [ ] Local storage layout compatibility: `storage`, `backups`, `.autocert_cache`, `.notify`, `.pb_temp_to_delete`.
- [ ] S3-backed storage and backup filesystem.

## API Envelope And Middleware

- [~] JSON error envelope.
- [ ] Exact PocketBase API error shapes and validation error data.
- [~] Request body limit middleware; route-specific overrides remain pending.
- [~] CORS middleware/settings.
- [ ] Gzip middleware.
- [ ] Rate-limit middleware and settings reload.
- [ ] Activity logging middleware.
- [ ] Auth loading middleware for records and superusers.
- [ ] IP whitelist enforcement for superusers.
- [ ] Trusted proxy / real client IP resolution.

## Health API

- [x] `GET /api/health`.
- [ ] Exact response metadata and failure modes.

## Collections API

Routes from `apis/collection.go`:

- [x] `GET /api/collections`
- [x] `POST /api/collections`
- [x] `GET /api/collections/{collection}`
- [x] `PATCH /api/collections/{collection}`
- [x] `DELETE /api/collections/{collection}`
- [x] `DELETE /api/collections/{collection}/truncate`
- [x] `PUT /api/collections/import`
- [x] `GET /api/collections/meta/scaffolds`
- [x] `GET /api/collections/meta/oauth2-providers`
- [x] `POST /api/collections/meta/dry-run-view`
- [x] Superuser-only access control for collection management.
- [ ] System collection protection.
- [ ] Collection name/id lookup parity.
- [ ] Collection schema validation parity.
- [~] Base collections.
- [~] Auth collections.
- [ ] View collections with SQL query validation and read-only records.
- [~] Rule fields: `listRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`.
- [ ] Auth collection `authRule` and `manageRule`.
- [ ] Index definitions and migration of indexes.

## Field Types And Validation

Field structs found under `core/field_*.go`:

- [~] `text`
- [~] `number`
- [~] `bool`
- [~] `email`
- [~] `url`
- [~] `date`
- [~] `password`
- [~] `select`
- [~] `file`
- [~] `relation`
- [~] `json`
- [~] `editor`
- [~] `autodate`
- [~] `geoPoint`
- [~] Common field properties: `id`, `name`, `system`, `hidden`, `required`, `presentable`, `help`.
- [~] Field-specific settings and validation: min/max, pattern, values, cascade delete, mime types, max size, max select, primary key, autogenerate pattern.
- [ ] Field modifiers such as `:autogenerate`, file append/prepend/subtract behavior, uploaded/unsaved file handling.
- [ ] System field immutability and reserved field names.

## Records CRUD API

Routes from `apis/record_crud.go`:

- [x] `GET /api/collections/{collection}/records`
- [x] `GET /api/collections/{collection}/records/{id}`
- [x] `POST /api/collections/{collection}/records`
- [x] `PATCH /api/collections/{collection}/records/{id}`
- [x] `DELETE /api/collections/{collection}/records/{id}`
- [x] Basic pagination response.
- [~] Basic filter support, including simple `&&`/`||` conjunctions.
- [ ] Full PocketBase filter language.
- [~] Full sort support.
- [~] `page`, `perPage`, `skipTotal`, `sort`, `filter`, `expand`, `fields` query params.
- [ ] Relation expansion and nested expansion.
- [ ] Back relation expansion.
- [ ] View collection record lookup.
- [ ] Hidden field handling.
- [ ] Auth record email visibility behavior.
- [ ] Record enrich hooks.
- [ ] Record validation hooks.
- [ ] Model create/update/delete hooks and transactional ordering.
- [ ] File cleanup on update/delete.
- [ ] Batch create/update/delete integration.

## Rule Language

- [~] Simple boolean/auth/id equality rules.
- [ ] Full tokenizer/parser/evaluator from `tools/search`.
- [ ] SQL-backed rule execution.
- [ ] `@request.*` context: auth, body, query, headers, method, context.
- [ ] Collection and relation field resolver.
- [ ] Multi-match relation/subquery support.
- [ ] Rule validation during collection save.
- [ ] Rule behavior parity for list/view/create/update/delete/auth/manage/MFA.

## Auth Records

Routes from `apis/record_auth.go`:

- [x] `GET /api/collections/{collection}/auth-methods`
- [x] `POST /api/collections/{collection}/auth-with-password`
- [x] `POST /api/collections/{collection}/auth-refresh`
- [~] `POST /api/collections/{collection}/auth-with-oauth2`
- [x] `POST /api/collections/{collection}/request-otp`
- [x] `POST /api/collections/{collection}/auth-with-otp`
- [x] `POST /api/collections/{collection}/request-password-reset`
- [x] `POST /api/collections/{collection}/confirm-password-reset`
- [x] `POST /api/collections/{collection}/request-verification`
- [x] `POST /api/collections/{collection}/confirm-verification`
- [x] `POST /api/collections/{collection}/request-email-change`
- [x] `POST /api/collections/{collection}/confirm-email-change`
- [x] `POST /api/collections/{collection}/impersonate/{id}`
- [ ] `GET|POST /api/oauth2-redirect`
- [~] Password hash storage and verification.
- [~] JWT-like auth token creation and verification.
- [ ] PocketBase token claims and token secret rotation.
- [~] Token types: `auth`, `file`, `verification`, `passwordReset`, `emailChange`.
- [x] Auth token refreshability.
- [x] Password reset token flow.
- [x] Verification token flow.
- [x] Email change token flow.
- [ ] Auth origins tracking.
- [~] External auth records stored on auth records as `externalAuths`.
- [ ] Auth alert email on new device/login.
- [x] Auth method discovery response.
- [~] Auth collection options: password auth, OAuth2 PKCE metadata, MFA, OTP, token durations, email templates.

## Superusers

- [ ] Built-in `_superusers` collection.
- [~] Superuser auth via records/tokens.
- [x] Superuser-only API protection for settings, logs, backups, crons, collection management, impersonation.
- [ ] Superuser IP whitelist setting.
- [ ] Superuser OTP command/API integration.
- [ ] Superuser file tokens for backup download.

## OAuth2 And OIDC Providers

Provider files found under `tools/auth`:

- [ ] Apple
- [ ] Bitbucket
- [ ] Box
- [ ] Discord
- [ ] Facebook
- [ ] Gitea/Forgejo
- [ ] Gitee
- [ ] GitHub
- [ ] GitLab
- [ ] Google
- [ ] Instagram
- [ ] Kakao
- [ ] Lark
- [ ] Linear
- [ ] LiveChat
- [ ] Mailcow
- [ ] Microsoft
- [ ] Monday
- [ ] Notion
- [ ] OIDC generic
- [ ] Patreon
- [ ] Planning Center
- [ ] Spotify
- [ ] Strava
- [ ] Trakt
- [ ] Twitch
- [ ] Twitter/X
- [ ] VK
- [ ] WakaTime
- [ ] Yandex
- [ ] OAuth2 provider config storage, secrets redaction, mapped fields, raw user payloads.
- [ ] OAuth2 state/subscription redirect flow.

## OTP And MFA

- [~] `_otps` system collection/model (currently stored in durable auth-flow token records).
- [x] OTP creation, storage, expiry, deletion.
- [~] OTP email template and mail send flow (RFC 5322/MIME body is built with `edgerun-protocols`; SMTP transport path uses `edgerun-email` when configured).
- [x] `request-otp` and `auth-with-otp`.
- [ ] MFA collection options.
- [ ] MFA challenge issuance and expiry.
- [ ] MFA auth method composition requirement.
- [ ] MFA rule evaluation.

## Files API

Routes from `apis/file.go`:

- [x] `POST /api/files/token`
- [x] `GET /api/files/{collection}/{recordId}/{filename}`
- [~] Multipart file upload on record create/update.
- [~] File token authorization.
- [ ] Thumbnail generation/query params.
- [x] Protected file behavior via record view rules.
- [ ] View collection file resolution.
- [ ] File field max count, max size, MIME validation.
- [ ] File name normalization and uniqueness parity.
- [ ] S3 file storage.
- [~] File deletion on record delete; update-time file diff cleanup remains pending.

## Realtime API

Routes from `apis/realtime.go`:

- [~] `GET /api/realtime`
- [x] `POST /api/realtime`
- [~] Server-sent events connection lifecycle.
- [x] Client id assignment and initial PB_CONNECT message.
- [~] Subscription registry/broker.
- [x] Subscription authorization by collection rules.
- [~] Record create/update/delete/auth events are persisted and returned to matching realtime subscriptions; live SSE delivery loop remains pending.
- [ ] Collection update/delete events.
- [ ] Auth refresh on realtime subscription updates.
- [ ] Realtime hooks: connect, subscribe, message send.

## Batch API

Route from `apis/batch.go`:

- [x] `POST /api/batch`
- [x] Atomic transactional batch behavior.
- [x] Per-request CRUD execution.
- [x] Body limit handling.
- [ ] Error response parity and rollback.

## Backups API

Routes from `apis/backup.go`:

- [x] `GET /api/backups`
- [x] `POST /api/backups`
- [x] `POST /api/backups/upload`
- [x] `GET /api/backups/{key}`
- [x] `DELETE /api/backups/{key}`
- [x] `POST /api/backups/{key}/restore`
- [~] Durable backup files.
- [ ] Zip archive format containing full `pb_data`.
- [ ] Exclude temp/autocert/backups/lost+found paths.
- [ ] Single active backup/restore lock.
- [~] Superuser auth and file-token download behavior.
- [~] Restore validation and rollback.
- [~] Backup name validation.
- [ ] S3 backup filesystem support.
- [ ] Backup hooks.

## Settings API

Routes from `apis/settings.go`:

- [x] `GET /api/settings`
- [x] `PATCH /api/settings`
- [~] `POST /api/settings/test/s3`
- [~] `POST /api/settings/test/email`
- [x] `POST /api/settings/apple/generate-client-secret`
- [~] Settings persistence and reload hooks.
- [~] Meta settings.
- [~] Logs settings.
- [~] SMTP settings.
- [~] S3 storage settings.
- [~] S3 backups settings.
- [~] Backups settings.
- [~] Trusted proxy settings.
- [~] Rate limit settings.
- [~] Batch settings.
- [~] Superuser IPs.
- [~] Email templates.

## Logs API

Routes from `apis/logs.go`:

- [x] `GET /api/logs`
- [x] `GET /api/logs/stats`
- [x] `GET /api/logs/{id}`
- [x] Request activity log storage.
- [~] Log retention/cleanup.
- [~] Logs filters/sorts/stats.
- [x] Superuser access control.

## Cron API And Scheduler

Routes from `apis/cron.go`:

- [x] `GET /api/crons`
- [x] `POST /api/crons/{id}`
- [~] Cron job registry.
- [ ] Schedule parser.
- [x] Manual cron execution.
- [ ] Cron hooks and logging.

## Mail

- [~] SMTP mailer via `edgerun-email::smtp::SmtpClient`; full PocketBase mail templates/hooks remain pending.
- [ ] Sendmail mailer.
- [ ] HTML-to-text conversion.
- [~] Auth verification email.
- [~] Password reset email.
- [~] Email change confirmation email.
- [~] OTP email.
- [ ] Auth alert email.
- [ ] Test email send API.
- [ ] Mailer hooks.

## Hooks And Extension Surface

- [ ] Hook registry with handler ids, priority, bind/unbind.
- [ ] App hooks: bootstrap, serve, terminate, settings reload.
- [ ] Model hooks: validate, create/update/delete, execute, after success/error.
- [ ] Collection hooks mirroring model hooks.
- [ ] Record hooks mirroring model hooks.
- [ ] API request hooks for every public API family.
- [ ] Auth hooks.
- [ ] File hooks.
- [ ] Backup hooks.
- [ ] Mailer hooks.
- [ ] Realtime hooks.
- [ ] JS VM binding surface for hooks, router, forms, filesystem, crypto, mailer, app.

## Admin UI

- [~] Build/embed `ui/dist`; current admin surface is a Rust-generated `edgerun-ui-core` scene instead of PocketBase's bundled SPA.
- [~] Serve admin under `/_/`; HTML is a minimal canvas bridge and `/_/admin-scene.json` carries the generated UI scene.
- [~] UI API client compatibility for core action links, list/count data, and shadcn coverage metadata; full PocketBase admin workflows remain pending.
- [ ] Login/session handling.
- [ ] Collections/schema editor.
- [ ] Records browser/editor.
- [ ] Logs, settings, backups, crons, auth provider management.
- [ ] Extension UI injection.

## Compatibility Tests To Port Or Mirror

- [ ] API scenario tests from `apis/*_test.go`.
- [ ] Field validation tests from `core/field_*_test.go`.
- [~] Record model/auth/token tests.
- [ ] Collection model/import/scaffold/dry-run tests.
- [~] Realtime tests.
- [ ] Backup create/upload/download/delete/restore tests.
- [ ] Settings and mail test forms.
- [ ] Cron schedule/job tests.
- [ ] Search/filter/sort parser tests.
- [ ] Security JWT/crypto/encrypt/random tests.
- [ ] Filesystem local/S3 tests.
- [ ] Migration runner and migratecmd tests.

## Current EdgeRun Slice Summary

Implemented enough to exercise a small self-contained subset:

- Health endpoint.
- Superuser-gated collection CRUD, import, truncate, and metadata routes.
- Record CRUD with pagination, basic filter/sort, `&&`/`||` filter conjunctions, field projection, and batch execution.
- Basic signed-token auth with auth refresh, OAuth2 external-auth linking, OTP, file tokens, and auth method discovery.
- PBKDF2-HMAC-SHA256 password storage.
- Very small rule subset.
- Binary-safe multipart file write/read using `edgerun-protocols` HTTP multipart parsing.
- EdgeRun Storage `DerivedDb` canonical state in `data.db`, with typed append rows for collections, records, admins, settings, logs, crons, auth-flow tokens, migrations, and realtime events; most route mutations now write exact typed deltas plus the canonical state pointer, with encrypted blob mirrors for `store.json` and uploaded files.
- EdgeRun Email SMTP client path for configured test sends and auth-flow messages.
- EdgeRun DNS zone/query APIs backed by `edgerun-protocols::dns`.
- EdgeRun DHCP discover/request APIs backed by `edgerun-protocols::dhcp::DhcpServerCore`.
- HTTPS serving through `edgerun-node` TLS, with PEM cert/key loading and persistent self-signed autocert cache.
- ACME HTTP-01 challenge serving plus DNS-01 and TLS-ALPN-01 material planning through EdgeRun ACME helpers.
- Minimal embedded QuickJS hook runtime for JS return expressions.
- Realtime client registration, subscription authorization, and durable record/auth event backlog delivery.
- EdgeRun UI Core admin manifest with shadcn component coverage served from `/_/edgerun-ui-core.json`, plus a Rust-generated shadcn/admin scene at `/_/admin-scene.json` rendered by a minimal canvas bridge.
- Backup list/create/upload/get/delete/restore as JSON store snapshots.
- Migration list/create tracking.
- Durable settings, request logs, and manually executable cron registry.

This is not full PocketBase parity yet. The biggest missing foundations are
SQLite file/page compatibility, typed query execution over `DerivedDb`, and full query semantics, the full rule/filter language,
system/auth collections, full auth flows, realtime SSE, full QuickJS plugin bindings,
full ACME issuance/renewal, mail/S3 integrations, and embedded admin UI.
