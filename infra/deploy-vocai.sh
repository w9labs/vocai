# ============================================================
# VOCAI: Vocab+AI — Deployment Script for W9 Labs VPS
# Run: bash infra/deploy-vocai.sh
# ============================================================

set -e

echo "🚀 Deploying Vocai to W9 Labs VPS..."

# SSH to server
SSH="ssh -p 22001 root@ffm.w9.nu"

# 1. Create database
echo "📊 Creating w9_vocabai database..."
$SSH "docker exec w9-postgres psql -U w9_admin -d postgres -c 'CREATE DATABASE w9_vocabai;'" 2>/dev/null || echo "Database may already exist"

# 2. Backup current docker-compose
echo "💾 Backing up docker-compose.yml..."
$SSH "cp /opt/w9-labs/docker-compose.yml /opt/w9-labs/docker-compose.yml.bak.$(date +%Y%m%d)"

# 3. Add Vocai service to docker-compose (append before volumes section)
echo "📝 Adding Vocai service to docker-compose.yml..."
$SSH << 'EOF'
cd /opt/w9-labs

# Check if vocai service already exists
if ! grep -q "vocai:" docker-compose.yml; then
  # Insert before volumes section
  sed -i '/^volumes:/i\
  vocai:\
    image: ghcr.io/w9labs/vocai:latest\
    container_name: vocai\
    restart: unless-stopped\
    depends_on:\
      w9-postgres:\
        condition: service_healthy\
    environment:\
      - PORT=3010\
      - RUST_LOG=info\
      - VOCAI_BASE_URL=https://vocai.top\
      - DATABASE_URL=postgres://w9_admin:${POSTGRES_PASSWORD}@w9-postgres:5432/w9_vocabai\
      - NVIDIA_API_KEY=${NVIDIA_API_KEY}\
      - ISSUER_URL=https://db.w9.nu\
      - OAUTH_CLIENT_ID=vocai\
      - OAUTH_CLIENT_SECRET=${VOCAI_OAUTH_SECRET}\
    labels:\
      - "traefik.enable=true"\
      - "traefik.http.routers.vocai.rule=Host(`vocai.top`)"\
      - "traefik.http.routers.vocai.entrypoints=web"\
      - "traefik.http.services.vocai.loadbalancer.server.port=3010"\
    healthcheck:\
      test: ["CMD", "curl", "-f", "http://localhost:3010/api/health"]\
      interval: 30s\
      timeout: 10s\
      retries: 3\
      start_period: 10s\

' docker-compose.yml
  
  echo "✅ Vocai service added to docker-compose.yml"
else
  echo "⚠️  Vocai service already exists in docker-compose.yml"
fi
EOF

# 4. Add environment variables to .env
echo "🔑 Adding environment variables..."
$SSH << 'EOF'
cd /opt/w9-labs
if ! grep -q "NVIDIA_API_KEY=" .env; then
  echo "NVIDIA_API_KEY=\${NVIDIA_API_KEY} # set in .env" >> .env
  echo "VOCAI_OAUTH_SECRET=\${VOCAI_OAUTH_SECRET}" >> .env
  echo "✅ Environment variables added"
else
  echo "⚠️  Environment variables already exist"
fi
EOF

# 5. Pull and deploy
echo "🐳 Pulling Vocai image..."
$SSH "cd /opt/w9-labs && docker compose pull vocai"

echo "🔄 Starting Vocai service..."
$SSH "cd /opt/w9-labs && docker compose up -d vocai"

# 6. Wait for startup
echo "⏳ Waiting for Vocai to start..."
sleep 10

# 7. Health check
echo "🏥 Running health check..."
$SSH "curl -sf http://localhost:3010/api/health" 2>/dev/null && echo "✅ Vocai is healthy!" || echo "⚠️  Vocai may still be starting up"

# 8. Check logs
echo "📋 Recent logs:"
$SSH "docker logs vocai --tail 20"

echo ""
echo "🎉 Vocai deployment complete!"
echo "🌐 Visit: https://vocai.top"
echo "📊 Health: https://vocai.top/api/health"
echo ""
echo "Next steps:"
echo "1. Register 'vocai' as OAuth client in w9-db admin panel"
echo "2. Update VOCAI_OAUTH_SECRET with actual OAuth client secret"
echo "3. Configure DNS for vocai.top in Cloudflare (point to VPS IP)"
