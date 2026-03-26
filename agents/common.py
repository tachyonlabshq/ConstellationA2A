"""Shared utilities for Constellation agents."""

import asyncio
import logging
import os
import signal
import sys
import time
import uuid
from dataclasses import dataclass, field

from constellation import ConstellationAgent, AgentConfig, Message


def setup_logging(name: str) -> logging.Logger:
    """Configure and return a logger for an agent."""
    level = os.environ.get("LOG_LEVEL", "INFO").upper()
    logging.basicConfig(
        level=getattr(logging, level, logging.INFO),
        format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
    )
    return logging.getLogger(name)


def load_config(default_display_name: str) -> AgentConfig:
    """Load agent config from environment variables."""
    return AgentConfig(
        homeserver=os.environ["MATRIX_HOMESERVER"],
        username=os.environ["AGENT_USERNAME"],
        password=os.environ["AGENT_PASSWORD"],
        display_name=os.environ.get("AGENT_DISPLAY_NAME", default_display_name),
    )


def generate_task_id() -> str:
    """Generate a unique task ID."""
    return f"task-{uuid.uuid4().hex[:12]}"


@dataclass
class TaskInfo:
    """Tracks an in-progress task."""
    task_id: str
    description: str
    assigned_to: str
    requested_by: str
    status: str = "in_progress"
    created_at: float = field(default_factory=time.time)
    chain_next: str | None = None  # next step description for multi-step tasks
    chain_agent: str | None = None  # next agent for multi-step tasks


class BaseAgent:
    """Base class with common agent lifecycle management."""

    def __init__(self, name: str, display_name: str):
        self.name = name
        self.log = setup_logging(name)
        self.config = load_config(display_name)
        self.agent = ConstellationAgent(self.config)
        self.shutdown_event = asyncio.Event()
        self.room = None
        self.server_name = os.environ.get("MATRIX_SERVER_NAME", "constellation.local")

    async def start(self):
        """Connect to Matrix, join rooms, register signal handlers, and run."""
        loop = asyncio.get_running_loop()
        for sig in (signal.SIGTERM, signal.SIGINT):
            loop.add_signal_handler(sig, self._handle_signal)

        await self.agent.connect()
        self.log.info("%s connected to Matrix.", self.name.capitalize())

        rooms_env = os.environ.get("AUTO_JOIN_ROOMS", "")
        for room_alias in rooms_env.split(","):
            room_alias = room_alias.strip()
            if room_alias:
                self.room = await self.agent.join_room(room_alias)
                self.log.info("Joined room: %s", room_alias)

        self.register_handlers()

        self.log.info("%s running. Waiting for messages...", self.name.capitalize())
        await self.shutdown_event.wait()
        await self.agent.disconnect()
        self.log.info("%s stopped.", self.name.capitalize())

    def register_handlers(self):
        """Override in subclasses to register message handlers."""
        raise NotImplementedError

    def _handle_signal(self):
        self.log.info("Shutdown signal received, stopping gracefully...")
        self.shutdown_event.set()

    def run(self):
        """Entry point - run the agent."""
        try:
            asyncio.run(self.start())
        except KeyboardInterrupt:
            pass
        sys.exit(0)

    # ---- Message formatting helpers ----

    @staticmethod
    def format_mention(username: str, server_name: str) -> str:
        """Format an @-mention for a user."""
        user_id = f"@{username}:{server_name}"
        return f'<a href="https://matrix.to/#/{user_id}">@{username}</a>'

    @staticmethod
    def format_section(title: str, items: list[str]) -> str:
        """Format a titled section with bullet points."""
        lines = [f"**{title}**"]
        for item in items:
            lines.append(f"  - {item}")
        return "\n".join(lines)

    @staticmethod
    def format_code_block(code: str, language: str = "") -> str:
        """Wrap code in a markdown code block."""
        return f"```{language}\n{code}\n```"

    def make_metadata(self, task_id: str | None = None, **extra) -> dict:
        """Build constellation metadata dict for a message."""
        meta = {"agent": self.name}
        if task_id:
            meta["task_id"] = task_id
        meta.update(extra)
        return meta
