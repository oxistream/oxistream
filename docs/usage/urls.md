# YouTube-compatible URLs

Oxistream uses YouTube's URL layout. Redirect extensions such as LibRedirect can send YouTube links to your instance by swapping the host.

| URL | Opens |
| --- | --- |
| `/watch?v={id}` | Video. Optional `t=` (or `start=`) start time, `list=` playlist. |
| `/{id}` | Video (youtu.be style). Redirects to `/watch?v={id}`, keeping `t=`. |
| `/shorts/{id}` | Short |
| `/@{handle}`, `/@{handle}/{tab}` | Channel. Tabs: `videos`, `shorts`, `streams`, `playlists`, `search`. |
| `/channel/UC…`, `/channel/UC…/{tab}` | Channel by ID |
| `/playlist?list={id}` | Playlist: YouTube, local, or `LL` for liked videos |
| `/results?search_query={query}` | Search, with optional `type`, `upload_date`, `duration`, `sort` |
| `/feed/subscriptions`, `/feed/history`, `/feed/playlists`, `/feed/channels` | Feed, history, playlists, subscribed channels |
| `/party/{id}` | Join a watch party |

!!! tip "Example"

    `https://www.youtube.com/watch?v=VIDEO_ID&t=42` becomes `https://oxistream.example.com/watch?v=VIDEO_ID&t=42`.

Pasting a YouTube link into the search box opens it directly as well.
