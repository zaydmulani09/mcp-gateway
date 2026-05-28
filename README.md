# MCP Gateway

Rust reverse proxy/gateway for routing, auth, rate limiting, and cost tracking across MCP servers.

## Quickstart

```bash
git clone https://github.com/zaydmulani09/mcp-gateway
cd mcp-gateway
MCPGW_MASTER_SECRET=your-secret docker compose up --build
```

Gateway runs at `http://localhost:8080`.

### Health check

```bash
curl http://localhost:8080/health
# {"status":"ok","version":"0.1.0"}
```

### Add an MCP server

```bash
mcpgw server add
mcpgw server list
```

### View logs

```bash
mcpgw logs show --limit 50
mcpgw stats
```
