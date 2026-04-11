# Security

## Scope

gui-mcp provides direct access to the local desktop — mouse, keyboard, screen capture, clipboard, and window management. It is designed for **local, trusted use** by AI agents running on the same machine.

## Security Model

### Transport

- **stdio only** — no network listeners for the MCP protocol. The AI agent communicates via stdin/stdout with the server process.
- The web preview server (if enabled) binds to `127.0.0.1` only — not accessible from other machines.

### Web Preview Authentication

- Each web preview session generates a random UUID token at startup.
- All HTTP routes require `?token=<uuid>` — requests without a valid token receive `401 Unauthorized`.
- The token is only shared with the AI agent via the `gui_web_preview` tool response.

### Permissions

gui-mcp runs with the permissions of the user who launched it. It can:

- Capture anything visible on screen
- Send mouse clicks and keyboard input to any application
- Read and write the system clipboard
- Enumerate, focus, move, resize, and close windows

These are inherent to its purpose. Users should only run gui-mcp in environments where they trust the connected AI agent.

## Best Practices

- Do not expose the gui-mcp process or its stdin/stdout to untrusted processes.
- Do not forward the web preview URL/token to untrusted parties.
- Review AI agent actions when gui-mcp is connected — the agent can interact with any visible application.
- Consider running gui-mcp in a sandboxed desktop environment (VM, container with virtual display) for untrusted workloads.

## Reporting Vulnerabilities

If you discover a security vulnerability, please report it privately rather than opening a public issue. Contact the maintainers at the email listed in the repository or open a private security advisory on GitHub.
