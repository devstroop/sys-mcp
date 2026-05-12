# Security

## Scope

sys-mcp provides direct access to the local desktop — mouse, keyboard, screen capture, clipboard, window management, file system, shell/terminal access, and the ability to manage other MCP servers. It is designed for **local, trusted use** by AI agents running on the same machine.

## Security Model

### Transport

- **stdio or HTTP** — The MCP protocol runs over stdio by default, or optional HTTP server (binds to `127.0.0.1` only — not accessible from other machines).
- The web preview server (if enabled) binds to `127.0.0.1` only — not accessible from other machines.

### Web Preview Authentication

- Each web preview session generates a random UUID token at startup.
- All HTTP routes require `?token=<uuid>` — requests without a valid token receive `401 Unauthorized`.
- The token is only shared with the AI agent via the `gui_web_preview` tool response.

### MCP Hub Security

When using MCP Hub to manage other MCP servers:

- **Local MCP servers**: sys-mcp spawns MCP server processes on the local machine. These servers inherit the same user permissions as the sys-mcp process.
- **Passthrough**: Tool calls are forwarded to registered MCP servers. Be cautious about which MCP servers you register.
- **No remote tunneling by default**: MCP Hub operates locally. Remote scenarios (like RDP) require explicit configuration.

### Permissions

sys-mcp runs with the permissions of the user who launched it. It can:

- Capture anything visible on screen
- Send mouse clicks and keyboard input to any application
- Read and write the system clipboard
- Enumerate, focus, move, resize, and close windows
- Read, write, and delete files on the filesystem
- Execute shell commands and manage interactive terminal sessions
- Start and manage other MCP server processes

These are inherent to its purpose. Users should only run sys-mcp in environments where they trust the connected AI agent.

## Best Practices

- Do not expose the sys-mcp process or its stdin/stdout to untrusted processes.
- Do not forward the web preview URL/token to untrusted parties.
- Review AI agent actions when sys-mcp is connected — the agent can interact with any visible application.
- Be cautious about which MCP servers you register with the MCP Hub — they will run with your user permissions.
- Consider running sys-mcp in a sandboxed desktop environment (VM, container with virtual display) for untrusted workloads.
- When using `gui_shell_exec` or `gui_shell_open`, be aware that shell commands run with your user permissions.

## Reporting Vulnerabilities

If you discover a security vulnerability, please report it privately rather than opening a public issue. Contact the maintainers at the email listed in the repository or open a private security advisory on GitHub.