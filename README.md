# RelayKit

RelayKit is a language-neutral communication gateway for AI agents. It connects messaging platforms to any backend through small JSON protocols instead of binding the application to one SDK or programming language.

The first-party platform adapter is Telegram.

## Architecture

```text
Telegram Bot API
  -> relaykit serve
  -> normalized JSON event
  -> HTTP backend
  -> normalized reply JSON
  -> Telegram Bot API
```

Backends can be written in any language that can receive HTTP JSON.

## Quick start

Create a config file:

```toml
[telegram]
token_env = "TELEGRAM_BOT_TOKEN"

[backend]
url = "http://127.0.0.1:8080/relaykit/events"
```

Run RelayKit:

```sh
relaykit serve --config relaykit.toml
```

Send an outbound Telegram message directly:

```sh
relaykit telegram send --chat-id 123456 --text 'hello from RelayKit'
```

## Backend event protocol

RelayKit posts a normalized event to the configured backend URL:

```json
{
  "id": "telegram:123",
  "platform": "telegram",
  "chat": {
    "id": "123456",
    "kind": "dm",
    "title": null,
    "thread_id": null
  },
  "sender": {
    "id": "42",
    "username": "alice",
    "display_name": "Alice"
  },
  "message": {
    "id": "123",
    "text": "hello",
    "timestamp": 1787518050
  },
  "raw": { }
}
```

The backend may return replies immediately:

```json
{
  "messages": [
    {
      "text": "hello back",
      "reply_to_message_id": "123"
    }
  ]
}
```

An empty body, `204 No Content`, or an empty `messages` list means no reply.

## Development

Use the Nix dev shell for Rust tooling:

```sh
nix develop -c cargo fmt --check
nix develop -c cargo check
nix develop -c cargo clippy -- -D warnings
```

## License

Apache-2.0
