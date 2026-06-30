#!/usr/bin/env bash
set -euo pipefail

echo "Starting MCP Gateway..."
docker compose up --build -d
echo "Gateway running at http://localhost:8080"
echo "Health check:"
curl -sf http://localhost:8080/health | python3 -m json.tool || echo "  (curl not available)"
