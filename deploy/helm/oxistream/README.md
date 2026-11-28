# Oxistream Helm chart

Deploys Oxistream on Kubernetes 1.29+ with Helm 3 or 4. PostgreSQL 18 or newer is required and not part of the chart; the examples use [CloudNativePG](https://cloudnative-pg.io).

## Install

```sh
helm install oxistream ./deploy/helm/oxistream \
  --set image.repository=ghcr.io/OWNER/oxistream \
  --set database.url='postgres://oxistream:secret@postgres:5432/oxistream' \
  --set config.OXISTREAM_PUBLIC_URL=https://oxistream.example.com
```

`image.repository` points to the published image `ghcr.io/OWNER/oxistream`; the tag defaults to the chart's `appVersion`. Releases also publish the chart itself:

```sh
helm install oxistream oci://ghcr.io/OWNER/charts/oxistream --version X.Y.Z \
  --set image.repository=ghcr.io/OWNER/oxistream ...
```

Migrations run at startup. The first account to register becomes admin.

## With CloudNativePG

With the [CloudNativePG operator](https://cloudnative-pg.io/docs/current/installation_upgrade) installed:

```sh
kubectl apply -f deploy/helm/oxistream/examples/cnpg-cluster.yaml
kubectl create secret generic oxistream-proxy \
  --from-literal=proxy-secret="$(openssl rand -base64 48)"
helm install oxistream ./deploy/helm/oxistream \
  -f deploy/helm/oxistream/examples/values-cnpg.yaml \
  --set image.repository=ghcr.io/OWNER/oxistream
```

[`examples/cnpg-cluster.yaml`](examples/cnpg-cluster.yaml) creates a PostgreSQL 18 `Cluster` named `oxistream-db` with database and owner `oxistream`. `database.cnpgCluster: oxistream-db` reads the `uri` key of the operator-generated `oxistream-db-app` secret, which points to the read-write service. Raise `instances` to 3 for a primary with two standbys.

## Values

| Value | Default | Description |
| --- | --- | --- |
| `image.repository` | `""` | Required. |
| `image.tag` / `image.digest` | appVersion / `""` | The digest wins when set. |
| `replicaCount` | `1` | See [multiple replicas](#multiple-replicas). |
| `config` | `OXISTREAM_LOG_FORMAT: json` | Non-secret `OXISTREAM_*` variables, rendered into a ConfigMap. |
| `database.cnpgCluster` | `""` | CloudNativePG cluster name; uses `<name>-app`, key `uri`. |
| `database.existingSecret` / `existingSecretKey` | `""` / `uri` | Secret holding the database URL. |
| `database.url` | `""` | Database URL, stored in the chart's Secret. |
| `proxySecret.value` / `existingSecret` / `existingSecretKey` | generated | At least 32 characters. Generated on install and kept on upgrade if unset. |
| `oidcClientSecret.value` / `existingSecret` / `existingSecretKey` | `""` | Only for confidential OIDC clients. |
| `extraEnv`, `extraEnvFrom` | `[]` | Additional container environment. |
| `ingress.*` | disabled | `className`, `annotations`, `hosts`, `tls`. |
| `httpRoute.*` | disabled | Gateway API `HTTPRoute`; `parentRefs`, `hostnames`. |
| `autoscaling.*` | disabled | `HorizontalPodAutoscaler` on CPU and optionally memory. |
| `podDisruptionBudget.minAvailable` / `maxUnavailable` | `""` / `1` | Created when more than one replica can run. |
| `resources` | 100m / 128Mi, limit 512Mi | |
| `startupProbe`, `livenessProbe`, `readinessProbe` | `/healthz`, `/healthz`, `/readyz` | `/readyz` fails until the database is reachable and OIDC discovery succeeded. |
| `cache.sizeLimit` | `256Mi` | emptyDir for the RustyPipe cache at `/var/lib/oxistream`. |
| `podSecurityContext`, `securityContext` | non-root 65532, read-only root, no capabilities, `RuntimeDefault` seccomp | |
| `nodeSelector`, `tolerations`, `affinity`, `topologySpreadConstraints` | empty | Spread constraints get the pod selector when `labelSelector` is omitted. |
| `serviceAccount.*`, `service.*`, `podAnnotations`, `podLabels`, `extraVolumes`, `extraVolumeMounts` | | The usual. |

Pods roll when `config` or the chart's secret values change. Changes to existing secrets need `kubectl rollout restart`.

## Ingress

Watch parties use WebSockets and the stream proxy serves long, range-requested responses. Most controllers pass WebSocket upgrades by default; raise idle timeouts and disable response buffering where the controller has them (see [`examples/values-cnpg.yaml`](examples/values-cnpg.yaml) for ingress-nginx annotations). Set `OXISTREAM_PUBLIC_URL` to the external URL, and `OXISTREAM_TRUSTED_PROXY_HEADER` (e.g. `x-forwarded-for`) so rate limiting sees client IPs instead of the controller's. Over plain HTTP, sign-in needs `OXISTREAM_COOKIE_SECURE=false`.

## Multiple replicas

Replicas are stateless and coordinate through PostgreSQL, but:

- All replicas must share `OXISTREAM_PROXY_SECRET`. The chart's Secret does this; for GitOps tools that render with `helm template`, set `proxySecret.value` or `proxySecret.existingSecret` so it is not regenerated on every render.
- YouTube binds stream URLs to the IP that fetched them. All pods must leave through a single egress IP (one NAT gateway or egress gateway).
- Watch party sync uses each replica's wall clock, so nodes must be NTP-synced.
- Background jobs use advisory locks and replicas sync watch parties via `LISTEN`/`NOTIFY`. Connect to PostgreSQL directly (CloudNativePG's `-rw` service) or through a session-mode pooler, not a transaction-mode one.
- Each replica opens up to `OXISTREAM_DATABASE_MAX_CONNECTIONS` (default 10) connections.

## Metrics

Prometheus metrics are served at `/metrics` on the `http` port.
