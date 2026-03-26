# Constellation A2A

**Agent-to-Agent communication over Matrix**

[![CI](https://github.com/tachyon-labs-hq/constellation/actions/workflows/ci.yml/badge.svg)](https://github.com/tachyon-labs-hq/constellation/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Constellation A2A is a lightweight, Dockerized system where AI agents collaborate through a shared [Matrix](https://matrix.org/) chat server. Agents connect as native Matrix users via the Constellation SDK (Rust core + Python bindings via PyO3), communicating through @-mentions and structured task metadata.

## Architecture

```
                         ┌──────────────────────────────────────────┐
                         │            Docker Network                │
                         │                                          │
  ┌──────────┐           │  ┌──────────┐      ┌─────────────────┐  │
  │          │  Bridge   │  │ Conduit  │      │    Agent SDK    │  │
  │ Telegram ├──────────►│  │ Matrix   │◄────►│                 │  │
  │  User    │           │  │ Server   │      │  ┌───────────┐  │  │
  └──────────┘           │  │          │      │  │Coordinator│  │  │
                         │  │  :6167   │      │  └─────┬─────┘  │  │
                         │  │          │      │    delegates     │  │
                         │  │          │      │   ┌────┴────┐   │  │
                         │  │          │      │   │         │   │  │
                         │  │          │      │ ┌─┴──┐  ┌───┴┐  │  │
                         │  │          │      │ │Res.│  │Code│  │  │
                         │  │          │      │ └────┘  └────┘  │  │
                         │  └────┬─────┘      └─────────────────┘  │
                         │       │ :8448 (host)                     │
                         └───────┼─────────────────────────────────┘
                                 │
                          External access
```

[Conduit](https://conduit.rs/) is a lightweight Rust Matrix homeserver (~10 MB RAM). Agents connect as native Matrix users via the **Constellation SDK** (Rust core + Python bindings via PyO3).

## Features

- **Native Matrix protocol** -- agents are first-class Matrix users with E2EE support
- **Rust SDK with Python bindings** -- write agents in Rust or Python via PyO3
- **Coordinator pattern** -- a coordinator agent delegates tasks to specialist agents
- **Lightweight infrastructure** -- Conduit uses ~10 MB RAM; the entire stack runs on a single machine
- **Docker-first deployment** -- one command to start the full multi-agent system
- **CLI tooling** -- manage agents, rooms, and messages from the command line
- **Extensible** -- add new agents by dropping in a Python script and a Dockerfile

## Quick Start

```bash
# 1. Clone the repository
git clone https://github.com/tachyon-labs-hq/constellation.git
cd constellation

# 2. Configure environment
cp .env.example .env
# Edit .env with your own secrets

# 3. Run setup (builds Conduit, generates secrets, registers agents)
make setup
make register

# 4. Start all services
make up

# 5. Interact -- check server health, then start messaging
curl http://localhost:8448/_matrix/client/versions
```

## Project Structure

```
Constellation/
  docker-compose.yml            # Service orchestration
  docker-compose.prod.yml       # Production overrides
  .env.example                  # Environment template
  Makefile                      # Build & run commands
  conduit/
    Dockerfile                  # Conduit Matrix server image
    conduit.toml                # Server configuration
  sdk/
    Cargo.toml                  # Rust workspace
    constellation-core/         # Core Rust SDK
    constellation-py/           # PyO3 Python bindings
  agents/
    base.Dockerfile             # Shared multi-stage build
    common.py                   # Shared agent utilities
    coordinator/                # Task routing agent
    researcher/                 # Research agent
    coder/                      # Code generation agent
  cli/
    constellation_cli.py        # CLI management tool
  scripts/
    setup.sh                    # Bootstrap script
    register-agents.sh          # Agent account registration
    health-check.sh             # Server health check
    security-check.sh           # Security audit script
  examples/
    simple_agent.py             # Minimal agent example
    multi_agent_demo.py         # Multi-agent collaboration demo
  tests/
    integration/                # Integration test suite
  docs/
    specs/                      # Design specifications
```

## SDK Usage

### Rust

```rust
use constellation_core::{ConstellationAgent, AgentConfig, Message};

let config = AgentConfig {
    homeserver: "http://conduit:6167".into(),
    username: "my-agent".into(),
    password: "secret".into(),
    display_name: Some("My Agent".into()),
};

let mut agent = ConstellationAgent::new(config)?;
agent.connect().await?;
let room = agent.join_room("#constellation:constellation.local").await?;

agent.on_mention(|event| async move {
    agent.send_message(&room, Message::text("Hello!")).await?;
    Ok(())
});

agent.run_forever().await?;
```

### Python

```python
from constellation import ConstellationAgent, AgentConfig, Message

agent = ConstellationAgent(AgentConfig(
    homeserver="http://conduit:6167",
    username="my-agent",
    password="secret",
    display_name="My Agent",
))

await agent.connect()
room = await agent.join_room("#constellation:constellation.local")

@agent.on_mention
async def handle_mention(event):
    await agent.send_message(room, Message(body="Hello!"))

await agent.run_forever()
```

## Configuration

Environment variables (set in `.env`):

| Variable               | Default                   | Description                        |
|------------------------|---------------------------|------------------------------------|
| `REGISTRATION_SECRET`  | `change-me-in-production` | Conduit registration shared secret |
| `COORDINATOR_PASSWORD` | `coordinator-secret`      | Coordinator agent password         |
| `RESEARCHER_PASSWORD`  | `researcher-secret`       | Researcher agent password          |
| `CODER_PASSWORD`       | `coder-secret`            | Coder agent password               |

## CLI Usage

```bash
# Show available commands
python cli/constellation_cli.py --help

# Check server health
make health

# View running containers
make status

# Follow agent logs
make logs

# Open a shell in the Conduit container
make shell-conduit
```

## Security

**Development mode** (default): Open registration is enabled, passwords are set to defaults, and the server listens on localhost only. Suitable for local development and testing.

**Production mode**: Use `docker-compose.prod.yml` for hardened settings. Before deploying to production:

1. Change **all** default passwords and secrets in `.env`
2. Disable open registration on Conduit
3. Configure TLS termination (reverse proxy recommended)
4. Run `./scripts/security-check.sh` to audit your configuration
5. Restrict network access to the Matrix server port

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute to Constellation A2A.

## License

[MIT](LICENSE) -- Copyright 2026 Tachyon Labs HQ
