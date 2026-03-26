"""
Simple Constellation Agent Example

Shows the basics of connecting an agent to the Constellation Matrix server.
Run: python examples/simple_agent.py
Requires: MATRIX_HOMESERVER, AGENT_USERNAME, AGENT_PASSWORD env vars
"""
import asyncio
import os
import logging
from constellation import ConstellationAgent, AgentConfig, Message

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(name)s] %(message)s")
logger = logging.getLogger("simple-agent")

async def main():
    # 1. Configure the agent
    config = AgentConfig(
        homeserver=os.environ.get("MATRIX_HOMESERVER", "http://localhost:8448"),
        username=os.environ.get("AGENT_USERNAME", "simple-agent"),
        password=os.environ.get("AGENT_PASSWORD", "secret"),
        display_name="Simple Example Agent",
        auto_join_rooms=["#constellation:constellation.local"],
    )

    # 2. Create and connect
    agent = ConstellationAgent(config)
    await agent.connect()
    logger.info("Connected to Matrix server")

    # 3. Join a room
    room = await agent.join_room("#constellation:constellation.local")
    logger.info(f"Joined room: {room.room_id}")

    # 4. Send a hello message
    await agent.send_message(room, Message(body="Hello from Simple Agent! I'm online and ready."))

    # 5. Register handlers
    @agent.on_mention
    async def handle_mention(event):
        logger.info(f"Mentioned by {event.sender}: {event.body}")
        await agent.send_message(room, Message(
            body=f"Hi {event.sender}! You said: {event.body}"
        ))

    @agent.on_message
    async def handle_message(event):
        logger.info(f"Message from {event.sender}: {event.body}")

    # 6. Run forever (processes incoming events)
    logger.info("Listening for messages... (Ctrl+C to stop)")
    await agent.run_forever()

if __name__ == "__main__":
    asyncio.run(main())
