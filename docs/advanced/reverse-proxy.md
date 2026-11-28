# Reverse proxy

Terminate TLS in a reverse proxy and forward everything to port 8080. The app serves the SPA, the API (`/api`), the media proxy (`/proxy`) and the operational endpoints from that one port.

## Checklist

| Topic | What to do |
| --- | --- |
| Public URL | Set `OXISTREAM_PUBLIC_URL=https://oxistream.example.com`. Required for OIDC and when the proxy rewrites `Host`. Atom feed links use it too. |
| Cookies | Keep `OXISTREAM_COOKIE_SECURE=true` behind HTTPS. |
| Host | Pass the original `Host` header, or set `OXISTREAM_PUBLIC_URL`. |
| Client IP | Set `OXISTREAM_TRUSTED_PROXY_HEADER` to the header your proxy sets, e.g. `X-Forwarded-For`. |
| WebSockets | Forward `Upgrade` and `Connection` for `/api/v0/rooms/{id}/ws`, and allow long-lived connections. |
| Buffering | Do not buffer responses under `/proxy/`. Video is streamed with range requests. |
| Timeouts | Allow long reads, e.g. 1 h, for WebSockets and streams. |
| Metrics | `/metrics` has no authentication. Block it at the proxy if it must not be public. |

## Origin check

Every request other than GET, HEAD and OPTIONS needs an `Origin` header, or failing that a `Referer` header, whose origin matches either:

- the request's `Host` header, or
- `OXISTREAM_PUBLIC_URL`.

Otherwise the request fails with 403. If the proxy rewrites `Host` (e.g. to `127.0.0.1:8080`), set `OXISTREAM_PUBLIC_URL`. The only exempt endpoint is OIDC back-channel logout (`POST /api/v0/auth/oidc/backchannel-logout`), which the identity provider calls server to server.

## Cookies

The session cookie `oxistream_session` is `HttpOnly`, `SameSite=Lax` and `Secure`. Browsers drop `Secure` cookies over plain HTTP, except on `localhost`. To test without TLS on another host, set `OXISTREAM_COOKIE_SECURE=false`. The server logs a warning while it is off.

## Client IP

Rate limiting and the login limiter key on the client IP. IPv6 addresses are grouped by /64. Without `OXISTREAM_TRUSTED_PROXY_HEADER`, every client behind the proxy shares the proxy's limit.

When the header is set, the server takes its last value, i.e. the address your proxy appended.

!!! warning

    Only set it when every request reaches Oxistream through that proxy. Otherwise clients can pick their own rate-limit key by sending the header themselves.

## Examples

=== "Caddy"

    Caddy passes `Host`, sets `X-Forwarded-For`, handles WebSockets and streams responses without buffering by default.

    ```caddy
    oxistream.example.com {
    	reverse_proxy 127.0.0.1:8080
    }
    ```

=== "nginx"

    ```nginx
    # In the http block:
    map $http_upgrade $connection_upgrade {
        default upgrade;
        ''      close;
    }

    server {
        listen 443 ssl;
        server_name oxistream.example.com;
        # ssl_certificate ...

        location / {
            proxy_pass http://127.0.0.1:8080;
            proxy_http_version 1.1;
            proxy_set_header Host $host;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header Upgrade $http_upgrade; # (1)!
            proxy_set_header Connection $connection_upgrade;
            proxy_buffering off; # (2)!
            proxy_read_timeout 1h; # (3)!
            client_max_body_size 16m; # (4)!
        }
    }
    ```

    1.  WebSocket upgrade for watch parties (`/api/v0/rooms/{id}/ws`).
    2.  Video is streamed with range requests under `/proxy/`. Buffering delays playback.
    3.  Watch party WebSockets and long video responses stay open for a long time.
    4.  Subscription imports may be up to 5 MB.

=== "Traefik"

    Traefik passes `Host`, sets `X-Forwarded-For`, supports WebSockets and streams responses by default. Example with Docker labels:

    ```yaml
    services:
      oxistream:
        labels:
          traefik.enable: "true"
          traefik.http.routers.oxistream.rule: Host(`oxistream.example.com`)
          traefik.http.routers.oxistream.entrypoints: websecure
          traefik.http.routers.oxistream.tls.certresolver: letsencrypt
          traefik.http.services.oxistream.loadbalancer.server.port: "8080"
    ```

    If parties disconnect or long videos stop, check the entrypoint's `transport.respondingTimeouts`.

=== "Kubernetes ingress"

    See [Kubernetes](../setup/kubernetes.md#ingress).

In every case, set `OXISTREAM_PUBLIC_URL` and `OXISTREAM_TRUSTED_PROXY_HEADER=X-Forwarded-For` for Oxistream as well.
