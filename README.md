# toad-mcp

MCP (Model Context Protocol) server for the
[Primatif Toad](https://github.com/Primatif/Primatif_Toad) ecosystem.

## What It Does

`toad-mcp` transforms Toad from a passive file generator into a **live context
oracle** that AI agents can query directly. It exposes Toad's ecosystem
knowledge as MCP tools over the stdio transport, enabling any MCP-compatible
client (Windsurf, Cursor, Claude Desktop, etc.) to query project metadata in
real-time.

- **list_projects** — Return a filtered list of projects by name, tag, stack,
  activity tier, or VCS status.
- **get_project_detail** — Return full context for a single project, including
  its CONTEXT.md deep-dive.
- **search_projects** — Semantic search across project names, essence, tags,
  and taxonomy.
- **get_ecosystem_summary** — Return the system-prompt-tier overview of the
  entire ecosystem.
- **get_ecosystem_status** — Return ecosystem health with per-project git
  status, staleness, and activity tiers.
- **get_project_stats** — Return disk usage analytics with per-project sizes
  and bloat index.

All tools are **read-only and idempotent** — safe for repeated agent calls with
no side effects.

## Role in the Ecosystem

`toad-mcp` is a **thin adapter** between the MCP protocol and the existing
library crates. All business logic lives in `toad-core`, `toad-discovery`,
`toad-ops`, and `toad-manifest`. The server discovers the workspace, delegates
to library functions, and serializes results as JSON responses.

```text
toad-core      ──┐
toad-discovery ──┤
toad-manifest  ──┼── toad-mcp (MCP server binary)
toad-ops       ──┘       │
                         └── stdio transport ── AI agent (Windsurf, Cursor, etc.)
```

## Client Configuration

```json
{
  "mcpServers": {
    "toad": {
      "command": "toad-mcp",
      "args": []
    }
  }
}
```

## License

BUSL-1.1
