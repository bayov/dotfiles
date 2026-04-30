Discord Outbound (DM Notifier)
==============================

**Status:** Future (post-MVP). **Depends on:** `01-gmail-integration.md` (shared
`secrets.py`, `errors.py`, `registry.py`, `query.py`). **Note:** outbound only.
Inbound chat (gateway connection, per-user conversations) is its own larger PRD:
`05-chat-interface.md`.

What to build
-------------

A REST-only Discord client that sends a DM. Used by future heartbeat
notifications.

Files
-----

```
src/sb/scripts/integrations/
└── discord_client.py        # REST-only via httpx (no discord.py)
```

`pyproject.toml` additions: none beyond `httpx` (already brought in by Gmail
PRD). **Not** `discord.py` — outbound DMs don't need the gateway/WebSocket; that
lib forces an `asyncio.Client` lifecycle plus intents config that are pure
overhead for fire-and-forget DMs. The chat-interface PRD adds it.

Auth setup
----------

1. Discord Developer Portal → New Application → Bot → Enable bot.
2. Permissions: Send Messages (DMs only). Outbound-only — **no "Message Content
   Intent" needed**, no privileged intents at all.
3. Copy Bot Token → `secrets.toml [discord] bot_token`.
4. Find your Discord user ID: Settings → Advanced → Developer Mode → right-click
   your name → Copy ID → `secrets.toml [discord] user_id`.

```toml
# ~/.local/share/second-brain/<instance>/secrets.toml
[discord]
bot_token = "..."
user_id = "..."
```

```toml
# config.toml
[integrations]
enabled = ["gmail", "calendar", "discord"]
```

`discord_client.py`
-------------------

```python
import httpx
from .errors import IntegrationAuthError, MessageTooLong
from .secrets import load_secrets
from .shared_auth import retry

DM_LIMIT = 2000  # Discord hard cap


def notify(message: str) -> None:
    """Send `message` as a DM. Single-message primitive — no chunking.

    Raises:
        MessageTooLong: if len(message) > 2000.
        IntegrationAuthError: 401 from Discord (bad/revoked bot token).
        IntegrationTransientError: retries exhausted on 429/5xx.
    """
    if len(message) > DM_LIMIT:
        raise MessageTooLong(len(message), DM_LIMIT)

    s = load_secrets()
    headers = {"Authorization": f"Bot {s.discord.bot_token}"}

    # 1. Open (or fetch existing) DM channel
    channel = retry(
        lambda: httpx.post(
            "https://discord.com/api/v10/users/@me/channels",
            headers=headers,
            json={"recipient_id": s.discord.user_id},
        )
    )
    channel_id = channel.json()["id"]

    # 2. Send the message
    retry(
        lambda: httpx.post(
            f"https://discord.com/api/v10/channels/{channel_id}/messages",
            headers=headers,
            json={"content": message},
        )
    )
```

`query.py` subcommand
---------------------

```bash
sb-query discord notify "Action ready: reply to Alice re: Q3 launch"
# Exit code 2 + stderr "Discord DMs cap at 2000 chars; got <n>. Split and retry."
# on MessageTooLong, so a calling agent has what it needs to recover.
```

Tests
-----

- `discord_client.tests.py` — patch `httpx.post`; assert two-call flow,
  `MessageTooLong` raised at 2001 chars, `IntegrationAuthError` on 401, retry on
  429/5xx.

Acceptance — "done"
-------------------

- `[integrations].enabled` includes `"discord"`.
- `sb-query discord notify "hello"` lands in Hagai's DMs.
- Default suite passes with `httpx.post` mocked.

**Explicit non-scope:** any inbound message handling, any gateway/WS connection,
any conversational state. That's `05-chat-interface.md`.
