# Chat Interface (Conversational Access via Adapters)

**Status:** Future (post-MVP). **Depends on:** MVP, `03-discord-outbound.md`
(for token + user_id reuse), `04-heartbeat-and-reflection.md` (which introduces
the projection layer and `project_event()` dispatcher this PRD extends).
**Note:** This adds the gateway/inbound side of Discord. Outbound DMs from `03`
keep working unchanged; this PRD adds `discord.py` and an asyncio event loop to
serve interactive conversations. Adding Slack/Telegram/iMessage/SMS later is a
single file plus an env-var entry.

## What to build

A persistent chat surface — Discord today, others later — built on a
platform-agnostic core and a thin per-platform adapter. Conversation history is
event-sourced.

## Architecture

Three layers, top-to-bottom:

1. **Adapters** (`adapters/*.py`) — platform-specific I/O only. Receive incoming
   messages, do auth, emit a normalized `ChatMessage`, send replies via a
   platform-native callback.
2. **Core** (`core.py`) — platform-agnostic. Takes a `ChatMessage`, loads
   history, calls Claude, persists turns as `chat_message` events, returns reply
   text.
3. **Runner** (`run.py`) — reads enabled adapters from env, runs them
   concurrently against the same core.

Each conversation has a stable, platform-namespaced `session_id` (e.g.
`discord:741...`, `slack:C0X...`).

## Key files

```
src/sb/chat/
├── run.py                    # entry point, exposed as `sb-chat`
├── core.py                   # platform-agnostic Claude loop
├── session_store.py          # DB-backed history (chat_messages projection)
├── adapter.py                # ChatAdapter protocol + ChatMessage dataclass
└── adapters/
    ├── __init__.py           # ADAPTERS = {"discord": DiscordAdapter, ...}
    ├── discord_adapter.py
    ├── slack_adapter.py      # (future)
    └── telegram_adapter.py   # (future)
```

`pyproject.toml` adds `discord.py` (gateway/inbound now needs it).

## `adapter.py` — the contract

```python
@dataclass
class ChatMessage:
    platform: str  # 'discord', 'slack', ...
    session_id: str  # f"{platform}:{channel_or_thread_id}"
    sender_id: str  # platform-native user id (already auth-checked)
    text: str


MessageHandler = Callable[[ChatMessage], Awaitable[str]]


class ChatAdapter(Protocol):
    name: str

    async def run(self, handle: MessageHandler) -> None:
        """Start the platform event loop. For each authorized incoming message,
        call `handle(msg)` and send the returned string back to the sender."""
```

Adapter responsibilities (the contract, not optional):

- **Auth** — drop messages from anyone who isn't Hagai before invoking `handle`.
- **DMs only** — ignore channel/group messages unless explicitly opted in.
- **Session ID namespacing** — always prefix with `{platform}:`.
- **Idempotent reply** — on transient send failures, do not double-invoke
  `handle`.

## `core.py`

```python
from anthropic import Anthropic

anthropic = Anthropic()


def load_context() -> str:
    vault = Path.home() / "second-brain/work/ai-vault"
    return "\n\n".join(
        (vault / f).read_text() for f in ("SOUL.md", "USER.md", "MEMORY.md")
    )


async def handle(msg: ChatMessage) -> str:
    history = get_history(msg.session_id)
    history.append({"role": "user", "content": msg.text})

    write_event(
        Event(
            source=msg.platform,
            type="chat_message",
            payload={"role": "user", "content": msg.text},
            metadata={"session_id": msg.session_id, "sender_id": msg.sender_id},
        )
    )

    response = anthropic.messages.create(
        model="claude-sonnet-4-6",
        max_tokens=1024,
        system=load_context(),
        messages=history,
    )
    reply = response.content[0].text

    write_event(
        Event(
            source=msg.platform,
            type="chat_message",
            payload={"role": "assistant", "content": reply},
            metadata={"session_id": msg.session_id},
        )
    )
    return reply
```

## `adapters/discord_adapter.py`

```python
import discord, os
from ..adapter import ChatMessage, MessageHandler


class DiscordAdapter:
    name = "discord"

    async def run(self, handle: MessageHandler) -> None:
        intents = discord.Intents.default()
        intents.dm_messages = True
        intents.message_content = True
        client = discord.Client(intents=intents)
        owner_id = os.environ["DISCORD_USER_ID"]

        @client.event
        async def on_message(m):
            if m.author.bot or not isinstance(m.channel, discord.DMChannel):
                return
            if str(m.author.id) != owner_id:
                return
            msg = ChatMessage(
                platform="discord",
                session_id=f"discord:{m.channel.id}",
                sender_id=str(m.author.id),
                text=m.content,
            )
            reply = await handle(msg)
            await m.reply(reply)

        await client.start(os.environ["DISCORD_BOT_TOKEN"])
```

**Privileged intents.** Inbound chat needs Message Content Intent — toggle it in
the Discord Developer Portal and re-invite the bot. (The outbound-only PRD did
not need this.)

## `run.py` — fan out

```python
import asyncio, os
from .core import handle
from .adapters import ADAPTERS


async def main():
    enabled = os.environ.get("CHAT_ADAPTERS", "discord").split(",")
    await asyncio.gather(*[ADAPTERS[name]().run(handle) for name in enabled])


if __name__ == "__main__":
    asyncio.run(main())
```

## `chat_messages` projection

```sql
CREATE TABLE chat_messages (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id    INTEGER NOT NULL REFERENCES events(id),
    session_id  TEXT NOT NULL,        -- 'discord:741...', 'slack:C0X...'
    platform    TEXT NOT NULL,
    role        TEXT NOT NULL,        -- 'user' | 'assistant'
    content     TEXT NOT NULL,
    created_at  TIMESTAMP NOT NULL
);
CREATE INDEX chat_messages_session ON chat_messages(session_id, id);
```

`project_event()` gains one branch: `chat_message` → insert into
`chat_messages`. `get_history(session_id)` is
`SELECT role, content FROM chat_messages WHERE session_id = ? ORDER BY id`.

## Adding a new platform (the test of the abstraction)

1. Create `adapters/slack_adapter.py` implementing `ChatAdapter` (auth via Slack
   signing secret, session_id `slack:{channel_id}`).
2. Register in `adapters/__init__.py`.
3. Add creds to `.env`, set `CHAT_ADAPTERS=discord,slack`.
4. Restart the systemd unit. No core changes, no schema changes.

If a new platform requires a change to `core.py` or the schema, the contract is
wrong — fix the contract, not the platform.

## Run as a background service

Owned by `08-deployment-systemd.md`. Reference unit:

```ini
# ~/.config/systemd/user/second-brain-chat.service
[Service]
WorkingDirectory=%h/second-brain/work
Environment=SECOND_BRAIN_INSTANCE=%h/second-brain/work
ExecStart=uv run --project %h/dotfiles/second-brain sb-chat
EnvironmentFile=%h/second-brain/work/.env
Restart=always
```

## Acceptance — "done"

- `sb-chat` starts, accepts a DM from Hagai's Discord, responds with Claude
  output, persists both turns as `chat_message` events.
- `chat_messages` projection contains the same turns; replay rebuilds it.
- A second Hagai-DM in the same channel sees prior history in the system prompt
  context.
- A non-Hagai DM is silently ignored.

**Dependencies:** MVP, Phase 4 (Discord token + user_id from
`03-discord-outbound.md`). **Estimated complexity:** High (first adapter); Low
per additional adapter.
