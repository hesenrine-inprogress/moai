# MOAI

**A workspace AI harness in your terminal.**

MOAI is a system-level AI harness built around a single idea: your AI should control your machine, not just talk to it. Voice-first, terminal-native, agent-driven — built to feel like infrastructure, not an app.

---

## What it does

- Talk to your PC — control it by voice, naturally
- Build and run agentic workflows in the background
- Plan and manage your work directly in the terminal
- Reach it anywhere — Telegram, WhatsApp, voice, all connected
- Long-term memory that actually knows you over time
- Skills that evolve and improve with use
- Deep system control — the layer between you and your machine

---

## Architecture

MOAI is split into two layers:

**Core — Rust**
The always-on foundation. Handles the TUI, voice I/O, agent dispatch, and onboarding. Fast, clean, compiled to a single binary. No dependencies to manage, no runtime to install.

**Agents — Python**
The flexible side. Skills, LLM clients, memory, and channel integrations. Python because the entire AI ecosystem lives there — no point fighting it.

The two layers talk to each other. The core runs the show, the agents do the work.

---

## Roadmap

### Core
- [ ] TUI — terminal cockpit
- [ ] Agent dispatcher
- [ ] Onboarding
- [ ] Voice I/O

### Agents
- [ ] LLM clients
- [ ] Long-term memory
- [ ] Skills system
- [ ] Channels — Telegram, WhatsApp

---

## License

Apache 2.0 — see [LICENSE](./LICENSE).
