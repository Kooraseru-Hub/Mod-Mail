#!/bin/bash
set -e
exec >> /var/log/startup-script.log 2>&1

echo "[$(date)] Starting container deployment..."

# Read configuration from instance metadata
REGISTRY=$(curl -sf -H "Metadata-Flavor: Google" \
  "http://metadata.google.internal/computeMetadata/v1/instance/attributes/registry")
IMAGE=$(curl -sf -H "Metadata-Flavor: Google" \
  "http://metadata.google.internal/computeMetadata/v1/instance/attributes/container-image")
DISCORD_TOKEN=$(curl -sf -H "Metadata-Flavor: Google" \
  "http://metadata.google.internal/computeMetadata/v1/instance/attributes/discord-token")
GCP_PROJECT=$(curl -sf -H "Metadata-Flavor: Google" \
  "http://metadata.google.internal/computeMetadata/v1/instance/attributes/gcp-project")

echo "[$(date)] Authenticating Docker to Artifact Registry..."
ACCESS_TOKEN=$(curl -sf -H "Metadata-Flavor: Google" \
  "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token" \
  | python3 -c "import sys,json;print(json.load(sys.stdin)['access_token'])")
echo "$ACCESS_TOKEN" | docker login -u oauth2accesstoken --password-stdin "https://${REGISTRY}"

echo "[$(date)] Pulling image: $IMAGE"
docker pull "$IMAGE"

echo "[$(date)] Restarting container..."
docker stop discord-bot 2>/dev/null || true
docker rm   discord-bot 2>/dev/null || true

docker run -d \
  --name discord-bot \
  --restart=unless-stopped \
  -e DISCORD_TOKEN="$DISCORD_TOKEN" \
  -e FIRESTORE_PROJECT_ID="$GCP_PROJECT" \
  "$IMAGE"

echo "[$(date)] Container started successfully."
