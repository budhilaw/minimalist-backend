#!/bin/bash

# Script to prepare SQLx cache files for offline compilation
# This needs to be run with a live database connection

set -e

echo "🔧 Preparing SQLx cache for offline compilation..."

# Check if DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
    echo "❌ DATABASE_URL environment variable is required"
    echo "Example: export DATABASE_URL=postgresql://username:password@localhost:5432/database_name"
    exit 1
fi

# Check if sqlx CLI is installed
if ! command -v sqlx &> /dev/null; then
    echo "📦 Installing SQLx CLI..."
    cargo install sqlx-cli --no-default-features --features postgres
fi

# Run migrations first
echo "🚀 Running database migrations..."
sqlx migrate run

# Generate query cache
echo "⚡ Generating SQLx query cache..."
cargo sqlx prepare

echo "✅ SQLx cache files generated successfully!"
echo "📝 You can now commit the .sqlx/ directory to version control"
echo "🚀 CI/CD builds will now use offline compilation" 