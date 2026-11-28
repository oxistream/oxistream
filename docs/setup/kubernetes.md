# Kubernetes

The Helm chart in [`deploy/helm/oxistream`](https://github.com/oxistream/oxistream/tree/main/deploy/helm/oxistream) deploys Oxistream on Kubernetes 1.29 or newer, with Helm 3 or 4. The chart does not include PostgreSQL (18 or newer is required). The examples use [CloudNativePG](https://cloudnative-pg.io).

## Install

From the repository:

```sh
helm install oxistream ./deploy/helm/oxistream \
  --set image.repository=ghcr.io/OWNER/oxistream \
  --set database.url='postgres://oxistream:secret@postgres:5432/oxistream' \
  --set config.OXISTREAM_PUBLIC_URL=https://oxistream.example.com
```

Releases also publish the chart as an OCI artifact:

```sh
helm install oxistream oci://ghcr.io/OWNER/charts/oxistream --version X.Y.Z \
  --set image.repository=ghcr.io/OWNER/oxistream ...
```

`image.repository` is required. The image tag defaults to the chart's `appVersion`. Migrations run at startup. The first local account to register becomes admin.

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

[`examples/cnpg-cluster.yaml`](https://github.com/oxistream/oxistream/blob/main/deploy/helm/oxistream/examples/cnpg-cluster.yaml) creates a PostgreSQL 18 `Cluster` named `oxistream-db` with database and owner `oxistream`.

With `database.cnpgCluster: oxistream-db`, the chart reads the `uri` key of the operator-generated `oxistream-db-app` secret. That key points to the read-write service. Raise `instances` to 3 for a primary with two standbys.

[`examples/values-cnpg.yaml`](https://github.com/oxistream/oxistream/blob/main/deploy/helm/oxistream/examples/values-cnpg.yaml) runs two replicas behind ingress-nginx with TLS.

## Values

| Value | Default | Description |
| --- | --- | --- |
| `image.repository` | `""` | Required. |
| `image.tag` / `image.digest` | appVersion / `""` | The digest wins when set. |
| `replicaCount` | `1` | See [multiple replicas](#multiple-replicas). |
| `config` | `OXISTREAM_LOG_FORMAT: json` | Non-secret `OXISTREAM_*` variables from [Configuration](../configuration.md), rendered into a ConfigMap. Other keys, and the three secrets below, are rejected. |
| `database.cnpgCluster` | `""` | CloudNativePG cluster name. Uses `<name>-app`, key `uri`. |
| `database.existingSecret` / `existingSecretKey` | `""` / `uri` | Secret that holds the database URL. |
| `database.url` | `""` | Database URL, stored in the chart's Secret. |
| `proxySecret.value` / `existingSecret` / `existingSecretKey` | generated | At least 32 characters. If unset, a value is generated on install and kept on upgrade. |
| `oidcClientSecret.value` / `existingSecret` / `existingSecretKey` | `""` | Only for confidential OIDC clients. |
| `extraEnv`, `extraEnvFrom` | `[]` | Additional container environment. |
| `ingress.*` | disabled | `className`, `annotations`, `hosts`, `tls`. |
| `httpRoute.*` | disabled | Gateway API `HTTPRoute`: `parentRefs`, `hostnames`. |
| `autoscaling.*` | disabled | `HorizontalPodAutoscaler` on CPU, and optionally memory. |
| `podDisruptionBudget.minAvailable` / `maxUnavailable` | `""` / `1` | Created when more than one replica can run. |
| `resources` | 100m / 128Mi, limit 512Mi | |
| `startupProbe`, `livenessProbe`, `readinessProbe` | `/healthz`, `/healthz`, `/readyz` | `/readyz` fails until the database is reachable and OIDC discovery has succeeded. |
| `cache.sizeLimit` | `256Mi` | emptyDir for the RustyPipe cache at `/var/lib/oxistream`. |
| `podSecurityContext`, `securityContext` | non-root 65532, read-only root, no capabilities, `RuntimeDefault` seccomp | |
| `nodeSelector`, `tolerations`, `affinity`, `topologySpreadConstraints` | empty | Spread constraints get the pod selector when `labelSelector` is omitted. |
| `serviceAccount.*`, `service.*`, `podAnnotations`, `podLabels`, `extraVolumes`, `extraVolumeMounts` | | Standard chart values. |

The database URL is resolved in this order: `cnpgCluster`, then `existingSecret`, then `url`.

Pods roll when `config` or the chart's own secret values change. After changing an existing secret, run `kubectl rollout restart`.

## Ingress

Watch parties use WebSockets, and the stream proxy serves long responses with range requests. Most controllers pass WebSocket upgrades by default. Where the controller allows it, raise idle timeouts and turn off response buffering. [`examples/values-cnpg.yaml`](https://github.com/oxistream/oxistream/blob/main/deploy/helm/oxistream/examples/values-cnpg.yaml) shows the ingress-nginx annotations.

Also:

- Set `OXISTREAM_PUBLIC_URL` to the external URL.
- Set `OXISTREAM_TRUSTED_PROXY_HEADER` (e.g. `x-forwarded-for`), so rate limiting sees client IPs instead of the controller's.
- Over plain HTTP, sign-in needs `OXISTREAM_COOKIE_SECURE=false`.

See [Reverse proxy](../advanced/reverse-proxy.md) for the reasons.

## Multiple replicas

Replicas are stateless and coordinate through PostgreSQL. A few conditions apply:

- All replicas must share `OXISTREAM_PROXY_SECRET`. The chart's Secret takes care of this.
- YouTube binds stream URLs to the IP that fetched them. All pods must leave through a single egress IP, such as one NAT gateway or egress gateway.
- Nodes must be NTP-synced. Watch party sync uses each replica's wall clock.
- Background jobs use advisory locks, and replicas sync watch parties via `LISTEN`/`NOTIFY`. Connect to PostgreSQL directly (CloudNativePG's `-rw` service) or through a session-mode pooler. A transaction-mode pooler does not work.
- Each replica opens up to `OXISTREAM_DATABASE_MAX_CONNECTIONS` (default 10) connections.

!!! warning "GitOps"

    Tools that render with `helm template`, such as Argo CD, would regenerate the proxy secret on every render. Set `proxySecret.value` or `proxySecret.existingSecret` explicitly.

Details are in [Replicas and rate limiting](../advanced/scaling.md#multiple-replicas).

## Metrics

Prometheus metrics are served at `/metrics` on the `http` port. See [Monitoring](../administration/monitoring.md#metrics).
