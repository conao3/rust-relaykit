<p align="center">
  <img src="docs/assets/banner.png" alt="RelayKit, a language-neutral communication gateway for AI agents" width="880">
</p>

<p align="center"><strong>A language-neutral communication gateway for AI agents</strong></p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/github/license/conao3/rust-relaykit" alt="License: Apache-2.0"></a>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/Rust-edition%202024-orange" alt="Rust edition 2024"></a>
  <a href="https://github.com/conao3/rust-relaykit/stargazers"><img src="https://img.shields.io/github/stars/conao3/rust-relaykit?style=flat" alt="GitHub stars"></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#cli">CLI</a> ·
  <a href="#backend-event-protocol">Backend protocol</a> ·
  <a href="#development">Development</a>
</p>

# RelayKit

RelayKit connects messaging platforms to an AI agent backend that you write in any language. RelayKit and your backend talk through small JSON protocols, so your application depends on no platform SDK.

The first-party platform adapter is Telegram. RelayKit polls the Telegram Bot API, sends each message to your backend as a normalized JSON event, and posts your backend's replies back to the chat.

## Architecture

```text
Telegram Bot API
  -> relaykit gateway start-gateway
  -> normalized JSON event
  -> HTTP backend
  -> normalized reply JSON
  -> Telegram Bot API
```

You can write the backend in any language that receives HTTP JSON. A command that reads and writes JSON lines on stdin and stdout also works as a backend.

## Install

RelayKit is a Rust crate with one binary, `relaykit`. Build and install it with Cargo:

```sh
cargo install --git https://github.com/conao3/rust-relaykit
```

## Quick start

Create a config file:

```toml
[telegram]
token_env = "TELEGRAM_BOT_TOKEN"

[backend]
url = "http://127.0.0.1:8080/relaykit/events"
```

For a JSONL CLI backend, set `command` instead of `url`:

```toml
[telegram]
token_env = "TELEGRAM_BOT_TOKEN"

[backend]
command = ["python3", "agent.py"]
```

RelayKit writes one normalized event JSON object plus a newline to the backend process stdin. The backend writes one reply batch JSON object plus a newline to stdout.

Backend stdin JSONL:

```json
{"id":"telegram:123","platform":"telegram","chat":{"id":"123456","kind":"dm","title":null,"thread_id":null},"sender":null,"message":{"id":"123","text":"hello","timestamp":1787518050},"raw":{}}
```

Backend stdout JSONL:

```json
{"messages":[{"text":"hello back","chat_id":null,"thread_id":null,"reply_to_message_id":"123"}]}
```

To send no reply, the backend writes an empty line or no line.

Run RelayKit:

```sh
relaykit gateway start-gateway --config-file relaykit.toml
```

Send an outbound Telegram message from the command line:

```sh
relaykit telegram send-message --config-file relaykit.toml --chat-id 123456 --text 'hello from RelayKit'
```

## CLI

The CLI takes input in the AWS CLI grammar: `relaykit [global options] <command> <subcommand> [--parameter value ...]`.

| Command | Parameters | Output |
|---|---|---|
| `gateway start-gateway` | `--config-file` (default `relaykit.toml`) | Runs until SIGINT, then prints `{"StoppedBy": "SIGINT"}` |
| `telegram send-message` | `--config-file` (default `relaykit.toml`), `--chat-id`, `--text`, `--thread-id` | `{"ChatId": "...", "ThreadId": null, "MessageId": "..."}` |

- Each parameter is a named `--kebab-case` option. RelayKit takes no positional arguments.
- RelayKit replaces a value starting with `file://` with the content of that file (for example `--text file://message.txt`). Read stdin with `file:///dev/stdin`.
- Global options: `--output json|text|table` (default `json`), `--debug` (debug logs on stderr), `--version`.
- `help` works at each level: `relaykit help`, `relaykit telegram help`, `relaykit telegram send-message help`.
- `gateway start-gateway` runs as a long-lived process. It writes its logs to stderr and prints its result when it stops.
- Exit codes: `0` success, `252` syntax or parameter error, `254` RelayKit cannot read the config, the config is incomplete, or Telegram rejected the call, `255` other failures.

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

The backend may return replies in the response body:

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
nix flake check --show-trace
nix develop -c cargo fmt --check
nix develop -c cargo check
nix develop -c cargo clippy --all-targets -- -D warnings
nix develop -c cargo test
```

## License

Apache-2.0. See [LICENSE](LICENSE).
