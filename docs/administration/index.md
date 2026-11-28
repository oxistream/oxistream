# Administration

The "Admin" area has two pages: `/admin` (Users) and `/admin/status` (Status). Only admins can open them. Everyone else gets "Admins only". Without an OIDC admin group, a local account that registers while there is no active admin becomes one, see [Authentication](../advanced/authentication.md#first-admin). SSO users become admins through the [admin group](../advanced/authentication.md#admin-role-from-groups).


Keeping the instance running is covered in [Upgrades and backups](upgrades.md) and [Monitoring](monitoring.md).

## Users

`/admin` lists users, newest first, 50 per page, with a username search.

| Column | Content |
| --- | --- |
| User | Username, marked "You" or "Disabled" where it applies. |
| Role | Admin or User. |
| Sign-in | Password (local) or SSO (OIDC). |
| Joined | Account creation. |
| Last seen | Most recent activity of any session. "Never" without sessions. |
| Subscriptions | Number of subscribed channels. |

Actions:

| Action | Effect |
| --- | --- |
| Make admin / Remove admin | Changes the role. With `OXISTREAM_OIDC_ADMIN_GROUP` set, an SSO user's role is synced again from the groups claim on their next sign-in. |
| Disable / Enable | A disabled user is signed out everywhere and cannot sign in. |
| Delete | Deletes the user with everything they own: subscriptions, playlists, history, settings, sessions. Cannot be undone. An SSO user who signs in again gets a new, empty account. |

Guards:

- Admins cannot change or delete their own account here. They use account deletion in their own settings instead.
- The last active admin cannot be demoted, disabled or deleted here. Sync from the OIDC admin group still demotes them when they are no longer in it.
- In their own settings, the last admin can delete their account only once no other users remain.

## Status

`/admin/status` shows the state of the replica that answers the request.

| Section | Fields |
| --- | --- |
| Server | Version, commit (if built with `OXISTREAM_GIT_SHA`), start time and uptime. |
| YouTube | `rustypipe-botguard` version (or not found), player cache entries on this replica. |
| Database | Reachable, PostgreSQL `server_version`. |
| Feed worker | Interval. Whether a run is in progress on any replica. Channels never fetched successfully (pending), channels whose last fetch failed (failing), channels due for the next run. Most recent fetch by any replica. This replica's last run: finish time, channels and playlists fetched or failed, and whether it stopped early because YouTube was blocking requests. Last database error of this replica's worker. |
| Data | Users, active admins, disabled users, channels with subscribers, subscriptions, playlists, cached videos. |
| Configuration | Local accounts, registration, login required, SSO enabled and discovered, SponsorBlock, trending region, rate limiting. |

Counts and feed fields are empty while the database is unreachable.

Things to look for:

- Botguard "not found": PO tokens are unavailable, and playback may fail. See [Botguard](../troubleshooting.md#botguard).
- Many failing channels, or a last run stopped by blocking: YouTube is throttling the instance's IP. See [YouTube blocks](../troubleshooting.md#youtube-blocks-and-403s).
- SSO enabled but not discovered: the provider is unreachable or the issuer is wrong. `/readyz` fails as well.

The same data is available as JSON from `GET /api/v0/admin/status`.
