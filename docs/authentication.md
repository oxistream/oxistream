# Authentication

Oxistream supports local accounts (username and password) and exactly one OpenID Connect provider. Each can be turned on or off. Sessions live in the database. There are no JWT sessions.

## Local accounts

| Variable | Default | Effect |
| --- | --- | --- |
| `OXISTREAM_AUTH_LOCAL_ENABLED` | `true` | Username and password sign-in. |
| `OXISTREAM_REGISTRATION_ENABLED` | `true` | `/register` is open. When `false`, it shows "Registration is closed". |

Rules:

- Usernames are 3 to 32 characters: letters, digits, `.`, `_` and `-`.
- Passwords are 12 to 256 characters. They are hashed with argon2id (19 MiB, 2 iterations, 1 lane).
- Changing the password signs out the account's other sessions.
- With rate limiting on, login and registration attempts are limited to a burst of 10 per client IP (one more every 6 s) and 5 per username (one more every 30 s).

With local accounts off and OIDC on, the instance is SSO-only.

## First admin

The first account to be created becomes admin, whether it registers locally or signs in through OIDC for the first time. A table lock prevents two first users. On a fresh instance, register right after the first start.

Admins manage other users in the [admin pages](administration.md).

## Sessions

- Cookie `oxistream_session`: a random 256-bit token. The database stores only its SHA-256, along with the user agent and IP at sign-in.
- The cookie is `HttpOnly`, `SameSite=Lax`, and `Secure` unless `OXISTREAM_COOKIE_SECURE=false`.
- Expiry slides: a session lasts `OXISTREAM_SESSION_TTL_DAYS` (default 30) days after its last use. Renewal happens at most every 5 minutes.
- Expired sessions are deleted every hour.
- State-changing requests must pass the [Origin check](reverse-proxy.md#origin-check).

## Require login

`OXISTREAM_REQUIRE_LOGIN=true` rejects anonymous requests to `/api/v0/*` and `/proxy/*` with 401. The SPA then sends every visitor to sign in.

These stay open:

- `/api/v0/auth/*`: sign-in, OIDC callback and back-channel logout.
- `/api/v0/instance`.
- `/api/v0/feed/atom/{token}`.
- `/healthz`, `/readyz`, `/metrics`.

Guests then cannot join watch parties either.

Without it (the default), anonymous visitors can search, watch and join parties. Subscriptions, playlists, history and starting parties need an account. See [what guests can do](user-guide.md#guests-and-accounts).

## OIDC

### Setup

1. Create a client at the provider using the authorization code flow. The client may be confidential or public. Oxistream always uses PKCE.
2. Register these URIs, with `{PUBLIC_URL}` being your `OXISTREAM_PUBLIC_URL`:

   | Setting | Value |
   | --- | --- |
   | Redirect URI | `{PUBLIC_URL}/api/v0/auth/oidc/callback` |
   | Post-logout redirect URI | `{PUBLIC_URL}/` |
   | Back-channel logout URL (optional) | `{PUBLIC_URL}/api/v0/auth/oidc/backchannel-logout`, with "session required" so that logout tokens carry `sid` |

3. Configure Oxistream:

   ```sh
   OXISTREAM_PUBLIC_URL=https://oxistream.example.com
   OXISTREAM_OIDC_ISSUER=https://id.example.com/realms/main
   OXISTREAM_OIDC_CLIENT_ID=oxistream
   OXISTREAM_OIDC_CLIENT_SECRET=...          # omit for public clients
   OXISTREAM_OIDC_NAME="Example ID"          # button: "Sign in with Example ID"
   OXISTREAM_OIDC_ADMIN_GROUP=oxistream-admins
   ```

All OIDC variables are listed in [Configuration](configuration.md#oidc).

Discovery runs in the background and retries with backoff (1 s, doubling up to 1 min). `/readyz` returns 503 "OIDC provider not discovered" until it succeeds. Local sign-in works in the meantime.

### Accounts

- A user's identity is the pair `(issuer, sub)`, never the email address. The first sign-in provisions a new account. OIDC users have no password.
- An OIDC identity is never linked to an existing local account, even when the names match.
- The username comes from the first claim that is present: `preferred_username`, `name`, the local part of `email`, or `sub`. It is sanitized to the username rules. If it is taken, `-2`, `-3`, … is appended.
- Claims that are missing from the ID token are fetched from the userinfo endpoint.
- `OXISTREAM_REGISTRATION_ENABLED=false` does not block OIDC provisioning. Restrict access at the provider instead.
- Disabled users cannot sign in.
- The pending login (state, nonce, PKCE verifier, return path) is kept for 10 minutes in the `oxistream_oidc` cookie, which is HMAC-signed with `OXISTREAM_PROXY_SECRET`. This lets any replica finish the login.

### Admin role from groups

With `OXISTREAM_OIDC_ADMIN_GROUP` set, the role is synced on every sign-in. Members of the group become admin, everyone else becomes a user. The groups are read from `OXISTREAM_OIDC_GROUPS_CLAIM` (default `groups`) in the ID token, or from userinfo if the claim is missing there. Dotted paths reach nested claims.

The last active admin is never demoted by sync. Without an admin group, roles are managed only in the admin pages. With one, sync overrides changes made there on the user's next sign-in.

The value must match a claim value exactly:

| Provider | Typical claim | Notes |
| --- | --- | --- |
| Keycloak | `groups` (add a "Group Membership" mapper), or `realm_access.roles` for realm roles | With "Full group path" on, groups look like `/admins`. Use that exact value. |
| Authentik | `groups` (default `profile` scope mapping) | Group names as shown in Authentik. |
| Authelia | `groups` (request the `groups` scope) | Add `groups` to `OXISTREAM_OIDC_SCOPES`, e.g. `openid profile email groups`. |

Check your provider's documentation for the current mapper and scope names.

### Logout

`POST /api/v0/auth/logout` ends the local session. If the provider has an end-session endpoint, the response includes `end_session_url` with `id_token_hint`, and the frontend sends the browser there. The provider then returns the browser to `{PUBLIC_URL}/`.

### Back-channel logout

The provider can end sessions server to server with `POST /api/v0/auth/oidc/backchannel-logout` (form field `logout_token`). This endpoint is exempt from the Origin check and from require-login.

The logout token is verified as follows:

- The signature is checked against the provider's JWKS, using the algorithms from discovery. An unknown `kid` refetches the JWKS, at most every 30 s.
- `iss` and `aud` must match, and `typ` must be `logout+jwt` if it is present.
- `iat` must be at most 10 minutes old, with 60 s leeway.
- The `events` claim must be the back-channel logout event. `sid` or `sub` must be present, and `nonce` must be absent.
- Each `jti` is accepted only once. Recording the `jti` and ending the sessions happen in one transaction, so a failed attempt can be retried with the same token. Used `jti`s are kept until the acceptance window ends, and cleaned up hourly.

Effect:

- `sid` ends that session. If `sub` is also given, it ends that session only for that user.
- `sub` alone ends all of the user's sessions.

The endpoint answers 200 with `Cache-Control: no-store`. Invalid or replayed tokens get a 400 OAuth error.
