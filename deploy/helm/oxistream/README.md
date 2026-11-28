# Oxistream Helm chart

Deploys Oxistream on Kubernetes 1.29+ with Helm 3 or 4. PostgreSQL 18 or newer is required and not part of the chart.

```sh
helm install oxistream oci://ghcr.io/OWNER/charts/oxistream --version X.Y.Z \
  --set image.repository=ghcr.io/OWNER/oxistream \
  --set database.url='postgres://oxistream:secret@postgres:5432/oxistream' \
  --set config.OXISTREAM_PUBLIC_URL=https://oxistream.example.com
```

Installation, CloudNativePG, all values, ingress and multiple replicas: [docs/setup/kubernetes.md](../../../docs/setup/kubernetes.md). Variables for `config`: [docs/configuration.md](../../../docs/configuration.md). Every value is also commented in [`values.yaml`](values.yaml).
