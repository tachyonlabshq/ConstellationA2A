# Constellation Agents

Agents are autonomous programs that connect to the Constellation Matrix server as native Matrix users. They communicate through room messages, @-mentions, and structured task metadata.

## How Agents Work

Each agent is a long-running process that:

1. Authenticates with the Conduit Matrix homeserver using its credentials
2. Joins one or more Matrix rooms
3. Listens for incoming messages (typically @-mentions)
4. Processes messages and responds through the same Matrix room
5. Can delegate work to other agents by @-mentioning them

All inter-agent communication happens over the standard Matrix protocol -- there is no proprietary RPC layer. This means any Matrix client can observe and interact with the agents.

## Agent Lifecycle

```
Register account (setup.sh / register-agents.sh)
        |
        v
   Connect to Conduit
        |
        v
   Join room(s)
        |
        v
   ┌──► Listen for events ◄──┐
   │         |                │
   │         v                │
   │    Process message       │
   │         |                │
   │         v                │
   └── Send response ────────┘
```

1. **Registration** -- Agent accounts are created on Conduit during setup via `scripts/register-agents.sh`.
2. **Connection** -- On container start, the agent authenticates and obtains an access token.
3. **Room join** -- The agent joins configured rooms (typically `#constellation:constellation.local`).
4. **Event loop** -- The agent syncs with the server, processes incoming events, and sends responses.
5. **Shutdown** -- On container stop, the agent disconnects gracefully.

## Built-in Agents

### Coordinator (`agents/coordinator/`)

The entry point for user requests. The coordinator:
- Receives incoming messages from users or external bridges
- Analyzes the request and determines which specialist agent(s) to involve
- Delegates tasks by @-mentioning the appropriate agent
- Aggregates responses and reports results back to the user

### Researcher (`agents/researcher/`)

Handles research and information gathering tasks:
- Web search and information retrieval
- Document summarization
- Fact-checking and source verification
- Knowledge synthesis across multiple sources

### Coder (`agents/coder/`)

Handles code-related tasks:
- Code generation from natural language descriptions
- Code review and bug detection
- Refactoring suggestions
- Explaining code behavior

## Creating a Custom Agent

### Step 1: Create the agent directory

```bash
mkdir agents/my-agent
```

### Step 2: Write the agent script

Create `agents/my-agent/agent.py`:

```python
from constellation import ConstellationAgent, AgentConfig, Message
import os

agent = ConstellationAgent(AgentConfig(
    homeserver=os.environ.get("HOMESERVER_URL", "http://conduit:6167"),
    username="my-agent",
    password=os.environ["MY_AGENT_PASSWORD"],
    display_name="My Agent",
))

@agent.on_mention
async def handle(event):
    # Your logic here
    response = f"Received: {event.body}"
    await agent.send_message(event.room, Message(body=response))

if __name__ == "__main__":
    import asyncio
    asyncio.run(agent.run_forever())
```

### Step 3: Create a Dockerfile (or use the shared base)

Option A -- Use the shared `base.Dockerfile`:

```dockerfile
FROM constellation-base AS runtime
COPY agents/my-agent/ /app/
CMD ["python", "agent.py"]
```

Option B -- Copy an existing agent's Dockerfile and adapt it.

### Step 4: Add the service to `docker-compose.yml`

```yaml
  agent-my-agent:
    build:
      context: .
      dockerfile: agents/my-agent/Dockerfile
    environment:
      - HOMESERVER_URL=http://conduit:6167
      - MY_AGENT_PASSWORD=${MY_AGENT_PASSWORD}
    depends_on:
      conduit:
        condition: service_healthy
    restart: unless-stopped
```

### Step 5: Register the agent account

Add the agent to `scripts/register-agents.sh`, or register manually:

```bash
# Add MY_AGENT_PASSWORD to .env
echo 'MY_AGENT_PASSWORD=my-secret' >> .env

# Register the account
make register
```

### Step 6: Start

```bash
make up
```

Your agent will connect to Conduit, join the configured room, and start responding to @-mentions.
