# GCP Workload Identity Federation Setup

## What Needs to Be Fixed on GCP

The error indicates the Workload Identity Pool configuration needs verification/setup:

### 1. Verify Workload Identity Pool Exists
```bash
gcloud iam workload-identity-pools describe github-actions-pool \
  --project=mod-mail-490516 \
  --location=global
```

### 2. Verify OIDC Provider Configuration
```bash
gcloud iam workload-identity-pools providers describe github \
  --project=mod-mail-490516 \
  --location=global \
  --workload-identity-pool=github-actions-pool
```

**Expected output should show:**
- `issuer_uri`: `https://token.actions.githubusercontent.com`
- `attribute_mapping`: Should map GitHub claims to GCP attributes

If not set up, create it:
```bash
gcloud iam workload-identity-pools create github-actions-pool \
  --project=mod-mail-490516 \
  --location=global \
  --display-name="GitHub Actions Pool"

gcloud iam workload-identity-pools providers create-oidc github \
  --project=mod-mail-490516 \
  --location=global \
  --workload-identity-pool=github-actions-pool \
  --display-name="GitHub Provider" \
  --attribute-mapping="google.subject=assertion.sub,attribute.actor=assertion.actor,attribute.aud=assertion.aud,google.iam.workloadIdentityPoolProviderId=projects/376050965276/locations/global/workloadIdentityPools/github-actions-pool/providers/github" \
  --issuer-uri=https://token.actions.githubusercontent.com \
  --attribute-value-uri=https://github.com/
```

### 3. Verify Service Account

Service account must exist and have the right permissions:
```bash
gcloud iam service-accounts describe github-deploy-bot@mod-mail-490516.iam.gserviceaccount.com \
  --project=mod-mail-490516
```

If it doesn't exist:
```bash
gcloud iam service-accounts create github-deploy-bot \
  --project=mod-mail-490516 \
  --display-name="GitHub Deploy Bot"
```

### 4. Grant Service Account Permissions

The service account needs permissions to push to GCR and deploy to Cloud Run:

```bash
# Push to GCR
gcloud projects add-iam-policy-binding mod-mail-490516 \
  --member=serviceAccount:github-deploy-bot@mod-mail-490516.iam.gserviceaccount.com \
  --role=roles/storage.admin

# Deploy to Cloud Run
gcloud projects add-iam-policy-binding mod-mail-490516 \
  --member=serviceAccount:github-deploy-bot@mod-mail-490516.iam.gserviceaccount.com \
  --role=roles/run.admin

# Pass service account (for Cloud Run)
gcloud projects add-iam-policy-binding mod-mail-490516 \
  --member=serviceAccount:github-deploy-bot@mod-mail-490516.iam.gserviceaccount.com \
  --role=roles/iam.serviceAccountUser
```

### 5. Allow Workload Identity Federation Access

Allow your GitHub repository to impersonate the service account:

**For all repositories in your GitHub account:**
```bash
gcloud iam service-accounts add-iam-policy-binding \
  github-deploy-bot@mod-mail-490516.iam.gserviceaccount.com \
  --project=mod-mail-490516 \
  --role=roles/iam.workloadIdentityUser \
  --member='principalSet://iam.googleapis.com/projects/376050965276/locations/global/workloadIdentityPools/github-actions-pool/attribute.repository/*/attribute.environment/*'
```

**Or specifically for this repository:**
```bash
gcloud iam service-accounts add-iam-policy-binding \
  github-deploy-bot@mod-mail-490516.iam.gserviceaccount.com \
  --project=mod-mail-490516 \
  --role=roles/iam.workloadIdentityUser \
  --member='principalSet://iam.googleapis.com/projects/376050965276/locations/global/workloadIdentityPools/github-actions-pool/attribute.repository/kooraseru/Mod-Mail/*'
```

---

## Running Locally with act

### Setup

1. **Install act** (if not installed):
   ```bash
   brew install act  # macOS
   # See https://github.com/nektos/act for other OS
   ```

2. **Fill in `.local/secrets.json`**:
   ```bash
   # Edit .local/secrets.json with your actual values
   {
       "GCP_PROJECT_ID": "your-actual-project-id",
       "CLOUD_RUN_REGION": "us-central1",
       "DISCORD_TOKEN": "your-actual-discord-token"
   }
   ```

3. **Update `.env.local`**:
   ```bash
   cp .env.local.example .env.local
   # Edit .env.local with your values
   ```

### Run Local Workflow

```bash
# Run the local workflow (skips GCP auth)
act -j deploy -W .github/workflows/local.yml
```

This will:
- ✅ Build the Rust project
- ✅ Build the Docker image locally
- ✅ Display deployment commands

It skips GCP authentication to avoid credential issues during local testing.

---

## GitHub Secrets Required

Make sure these are set in your GitHub repository (Settings → Secrets and variables → Actions):
- `GCP_PROJECT_ID`: Your GCP project ID
- `CLOUD_RUN_REGION`: Cloud Run region (e.g., `us-central1`)
- `DISCORD_TOKEN`: Your Discord bot token

