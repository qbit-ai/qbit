# Architecture

High-level repo structure:

```text
qbit/
├── frontend/               # React 19 + TypeScript + Vite
│   ├── components/         # UI components
│   ├── hooks/              # Tauri event subscriptions
│   ├── lib/                # Typed invoke() wrappers
│   └── store/              # Zustand + Immer state
└── backend/crates/         # Rust workspace
    ├── qbit/               # Main app: Tauri commands, CLI
    ├── qbit-ai/            # Agent orchestration, LLM clients
    ├── qbit-core/          # Foundation types
    └── ...
```

Related docs:
- [Planning system](planning-system.md)
- [System hooks](system-hooks.md)
- [Tool use](tool-use.md)
