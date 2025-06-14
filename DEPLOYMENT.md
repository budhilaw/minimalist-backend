# Deployment Guide

This guide will help you deploy the minimalist-backend to your Ubuntu VPS with CI/CD using GitHub Actions.

## 🏗️ Architecture

- **Domain**: `api.budhilaw.com` (proxied through frontend nginx)
- **SSL**: Handled by frontend nginx container
- **Reverse Proxy**: Shared nginx container from frontend (portfolio-network)
- **Container Registry**: GitHub Container Registry (GHCR)
- **Database**: PostgreSQL 15
- **Cache**: Redis 7
- **CI/CD**: GitHub Actions

**Note**: This backend uses the shared nginx container from the frontend deployment. Both frontend and backend containers are connected via the `portfolio-network` Docker network.

## 📋 Prerequisites

- Ubuntu VPS (20.04 or 22.04)
- Domain managed by Cloudflare (`budhilaw.com`)
- GitHub repository with Actions enabled
- SSH access to your VPS

## 🚀 Initial VPS Setup

### 1. Run Setup Script on VPS

```bash
# Download and run the VPS setup script
curl -sSL https://raw.githubusercontent.com/yourusername/minimalist-backend/main/scripts/setup-vps.sh | bash

# Log out and back in to apply docker group changes
exit
# SSH back in
```

### 2. Clone Repository

```bash
# Generate SSH key if you haven't
ssh-keygen -t ed25519 -C "your-email@example.com"
cat ~/.ssh/id_ed25519.pub  # Add this to your GitHub SSH keys

# Clone repository
git clone git@github.com:yourusername/minimalist-backend.git /opt/minimalist-backend
cd /opt/minimalist-backend
```

### 3. Configure Environment

```bash
# Copy and edit environment file
cp env.production.example .env
nano .env

# Fill in these values:
# POSTGRES_PASSWORD=your-strong-postgres-password
# REDIS_PASSWORD=your-strong-redis-password
```

### 4. Configure Application Secrets

```bash
# Copy and edit secret configuration
cp example.secret.yaml .secret.yaml
nano .secret.yaml

# Update the database and redis URLs to match your .env passwords
```

### 5. Generate SQLx Cache Files (Required for CI/CD)

SQLx requires query cache files for offline compilation in CI/CD:

```bash
# Set up local database connection (use your actual database credentials)
export DATABASE_URL=postgresql://username:password@localhost:5432/your_db

# Generate SQLx cache files
chmod +x scripts/prepare-sqlx.sh
./scripts/prepare-sqlx.sh

# Commit the generated cache files to your repository
git add .sqlx/
git commit -m "Add SQLx query cache for offline compilation"
git push origin main
```

**Note**: This step is required before your first deployment, as the CI/CD pipeline uses these cache files to compile SQLx queries without a database connection.

## 🔐 SSL Certificate Setup

SSL certificates are handled by the frontend nginx container. Make sure your frontend deployment is properly configured with SSL for both the main domain and API subdomain.

The API will be accessible at `https://budhilaw.com/api/` through the frontend's nginx reverse proxy.

## ⚙️ GitHub Actions Setup

### 1. Configure Repository Secrets

In your GitHub repository, go to **Settings > Secrets and variables > Actions** and add:

```
VPS_HOST=your-vps-ip-address
VPS_USER=your-username
VPS_SSH_KEY=your-private-ssh-key
VPS_SSH_PORT=22
```

To get your SSH private key:
```bash
cat ~/.ssh/id_ed25519  # Copy the entire private key
```

### 2. Enable GitHub Container Registry

1. Go to your GitHub repository settings
2. Navigate to **Actions > General**
3. Under "Workflow permissions", select "Read and write permissions"

## 🚀 Deploy

### Automatic Deployment (Recommended)

Simply push to the main branch:

```bash
git add .
git commit -m "feat: setup deployment"
git push origin main
```

GitHub Actions will automatically:
1. Run tests and linting
2. Build Docker image
3. Push to GitHub Container Registry
4. Deploy to your VPS

### Manual Deployment (Emergency)

If you need to deploy manually:

```bash
cd /opt/minimalist-backend
chmod +x scripts/deploy.sh
./scripts/deploy.sh
```

## 🔍 Monitoring & Troubleshooting

### Check Service Status

```bash
cd /opt/minimalist-backend
sudo docker-compose -f docker-compose.prod.yml ps
```

### View Logs

```bash
# All services
sudo docker-compose -f docker-compose.prod.yml logs

# Specific service
sudo docker-compose -f docker-compose.prod.yml logs api
sudo docker-compose -f docker-compose.prod.yml logs postgres
sudo docker-compose -f docker-compose.prod.yml logs redis
sudo docker-compose -f docker-compose.prod.yml logs nginx
```

### Test API Health

```bash
curl https://budhilaw.com/api/v1/health
```

### SSL Certificate Status

SSL certificates are managed by the frontend deployment. Check the frontend container logs for SSL-related issues.

## 🛡️ Security Features

- **Rate Limiting**: Both application-level and Nginx-level
- **CORS**: Configured for your frontend domain
- **SSL/TLS**: Modern configuration with HSTS
- **Firewall**: UFW configured with minimal open ports
- **Fail2Ban**: Protection against brute force attacks
- **Container Security**: Non-root user, minimal attack surface

## 🔄 Updating

To update your deployment:

1. **Via GitHub Actions** (Recommended):
   ```bash
   git add .
   git commit -m "your changes"
   git push origin main
   ```

2. **Manual Update**:
   ```bash
   cd /opt/minimalist-backend
   ./scripts/deploy.sh
   ```

## 📊 API Endpoints

Your API will be available at:

- **Health Check**: `https://api.budhilaw.com/api/v1/health`
- **Authentication**: `https://api.budhilaw.com/api/v1/auth/`
- **Posts**: `https://api.budhilaw.com/api/v1/posts/`
- **Portfolio**: `https://api.budhilaw.com/api/v1/portfolio/`
- **Services**: `https://api.budhilaw.com/api/v1/services/`

## 🆘 Emergency Procedures

### Complete System Recovery

```bash
# Stop all services
cd /opt/minimalist-backend
sudo docker-compose -f docker-compose.prod.yml down

# Remove all containers and volumes (⚠️ This will delete your data!)
sudo docker system prune -a --volumes

# Redeploy
./scripts/deploy.sh
```

### Database Backup

```bash
# Create backup
sudo docker exec minimalist_postgres pg_dump -U kai personal-website > backup_$(date +%Y%m%d_%H%M%S).sql

# Restore backup
sudo docker exec -i minimalist_postgres psql -U kai personal-website < backup_file.sql
```

## 📞 Support

If you encounter issues:

1. Check the logs first
2. Verify all environment variables
3. Ensure SSL certificates are valid
4. Check Cloudflare DNS settings
5. Verify GitHub Actions secrets 