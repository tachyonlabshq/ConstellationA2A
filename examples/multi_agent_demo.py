"""
Multi-Agent Demo

Runs two agents in the same process. Agent A sends a task to Agent B,
Agent B processes it and responds. Good for testing without Docker.

Run: python examples/multi_agent_demo.py
"""
import asyncio
import os
import logging
from constellation import ConstellationAgent, AgentConfig, Message, Task

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(name)s] %(message)s")
logger = logging.getLogger("multi-agent-demo")

HOMESERVER = os.environ.get("MATRIX_HOMESERVER", "http://localhost:8448")
ROOM = "#constellation:constellation.local"

async def run_agent_b():
    """Agent B: responds to tasks from Agent A."""
    config = AgentConfig(
        homeserver=HOMESERVER,
        username="agent-bravo",
        password=os.environ.get("AGENT_B_PASSWORD", "bravo-secret"),
        display_name="Agent Bravo",
    )
    agent = ConstellationAgent(config)
    await agent.connect()
    room = await agent.join_room(ROOM)
    logger.info("[B] Online and listening")

    @agent.on_mention
    async def handle_mention(event):
        logger.info(f"[B] Got task from {event.sender}: {event.body}")
        # Simulate processing
        await asyncio.sleep(1)
        result = f"Processed: '{event.body}' -> Result: 42"
        await agent.send_message(room, Message(body=result))
        logger.info(f"[B] Sent result back")

    await agent.run_forever()

async def run_agent_a():
    """Agent A: sends a task to Agent B."""
    config = AgentConfig(
        homeserver=HOMESERVER,
        username="agent-alpha",
        password=os.environ.get("AGENT_A_PASSWORD", "alpha-secret"),
        display_name="Agent Alpha",
    )
    agent = ConstellationAgent(config)
    await agent.connect()
    room = await agent.join_room(ROOM)
    logger.info("[A] Online")

    # Wait a moment for B to connect
    await asyncio.sleep(2)

    # Send a task to Agent B via @-mention
    logger.info("[A] Sending task to Agent Bravo...")
    await agent.mention_agent(room, "agent-bravo", Message(
        body="Please analyze the quarterly data and summarize findings",
        metadata={"task_type": "analysis", "priority": "high"},
    ))

    @agent.on_message
    async def handle_response(event):
        if event.sender != agent.config.username:
            logger.info(f"[A] Got response: {event.body}")

    await agent.run_forever()

async def main():
    logger.info("Starting multi-agent demo...")
    await asyncio.gather(
        run_agent_b(),
        run_agent_a(),
    )

if __name__ == "__main__":
    asyncio.run(main())
