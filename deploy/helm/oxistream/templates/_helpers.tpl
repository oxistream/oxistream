{{- define "oxistream.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "oxistream.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{- define "oxistream.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "oxistream.labels" -}}
helm.sh/chart: {{ include "oxistream.chart" . }}
{{ include "oxistream.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{- define "oxistream.selectorLabels" -}}
app.kubernetes.io/name: {{ include "oxistream.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{- define "oxistream.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "oxistream.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{- define "oxistream.image" -}}
{{- $repo := required "image.repository is required" .Values.image.repository }}
{{- if .Values.image.digest }}
{{- printf "%s@%s" $repo .Values.image.digest }}
{{- else }}
{{- printf "%s:%s" $repo (default .Chart.AppVersion .Values.image.tag) }}
{{- end }}
{{- end }}

{{- define "oxistream.multiReplica" -}}
{{- if or .Values.autoscaling.enabled (gt (int .Values.replicaCount) 1) }}true{{ end }}
{{- end }}

{{/* Secret name and key for the database URL, as "name:key". */}}
{{- define "oxistream.databaseSecretRef" -}}
{{- $db := .Values.database }}
{{- if $db.cnpgCluster }}
{{- printf "%s-app:uri" $db.cnpgCluster }}
{{- else if $db.existingSecret }}
{{- printf "%s:%s" $db.existingSecret $db.existingSecretKey }}
{{- else if $db.url }}
{{- printf "%s:database-url" (include "oxistream.fullname" .) }}
{{- else }}
{{- fail "set database.cnpgCluster, database.existingSecret or database.url" }}
{{- end }}
{{- end }}

{{- define "oxistream.proxySecretRef" -}}
{{- if .Values.proxySecret.existingSecret }}
{{- printf "%s:%s" .Values.proxySecret.existingSecret .Values.proxySecret.existingSecretKey }}
{{- else }}
{{- printf "%s:proxy-secret" (include "oxistream.fullname" .) }}
{{- end }}
{{- end }}

{{- define "oxistream.oidcClientSecretRef" -}}
{{- if .Values.oidcClientSecret.existingSecret }}
{{- printf "%s:%s" .Values.oidcClientSecret.existingSecret .Values.oidcClientSecret.existingSecretKey }}
{{- else if .Values.oidcClientSecret.value }}
{{- printf "%s:oidc-client-secret" (include "oxistream.fullname" .) }}
{{- end }}
{{- end }}

{{/* Proxy secret for the chart's Secret: explicit value, else the one from a previous install, else random. */}}
{{- define "oxistream.proxySecretValue" -}}
{{- if .Values.proxySecret.value }}
{{- if lt (len .Values.proxySecret.value) 32 }}
{{- fail "proxySecret.value must be at least 32 characters" }}
{{- end }}
{{- .Values.proxySecret.value }}
{{- else }}
{{- $existing := lookup "v1" "Secret" .Release.Namespace (include "oxistream.fullname" .) }}
{{- if and $existing (index ($existing.data | default dict) "proxy-secret") }}
{{- index $existing.data "proxy-secret" | b64dec }}
{{- else }}
{{- randAlphaNum 64 }}
{{- end }}
{{- end }}
{{- end }}

{{- define "oxistream.secretEnv" -}}
{{- $ref := splitList ":" .ref -}}
- name: {{ .name }}
  valueFrom:
    secretKeyRef:
      name: {{ index $ref 0 }}
      key: {{ index $ref 1 }}
{{- end }}
