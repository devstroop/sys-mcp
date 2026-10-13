# JEV Integration Plan for sys-mcp

Status: draft · Created: 2026-10-12
Goal: add a bounded decision layer (TypeSafe JEV / System One) to sys-mcp, following the
architecture proven by the open-source JEV ecosystem, while keeping all permissions,
execution, and verification in deterministic Rust code.

---

## 1. Background

**JEV (Jev 1.13, "System One")** is TypeSafe AI's decision model. Instead of generating text it
evaluates a `state` against typed questions and returns values + calibrated probabilities:

| Type | Returns | Use for |
|---|---|---|
| `noul` | `{noul: 0..1}` | "Is this statement true?" — guardrails, gates, urgency |
| `choice` | `{choice, probabilities, confidence}` | pick one option from a bounded set |
| `score` | `{score, probabilities, confidence, legend}` | ordered rubric; `score` = expected value over legend indices |

- Endpoint: `POST /v1/systemone` (alias `/v1/decisions`), **not** OpenAI-compatible.
- Models: `jev-1.13-free` ($0, rate-limited) and `jev-1.13` ($0.042/M input, output free).
- Multiple questions per request are evaluated in parallel in isolation; ~300–500 ms per call.
- Limits: 64K tokens total input, 32K for state + longest question. Hosted only (no local weights).
- Local gateway verified working: `POST http://192.168.2.49:5050/v1/systemone`, `Bearer sk-dev-key`
  (both plain `noul` and combined `choice`+`score`+`noul` requests return 200 with typed answers).

### Design law (from every serious JEV project reviewed)

```
deterministic code observes → builds a BOUNDED action space → Jev picks ONE choice
→ local policy + freshness guard → code executes → code verifies → compact result out
```

JEV **never** owns: execution, permissions, free-text generation, or completion claims.
It replaces exactly one narrow fuzzy judgment per call.

---

## 2. References reviewed

| Reference | Primary lesson |
|---|---|
| [jev-browser](https://github.com/Ying-Kai-Liao/jev-browser) | Status vocabulary (`done/likely_done/needs_confirmation/error/stuck/ambiguous/blocked`), irreversible-action pause, block-ranking reads (batches of ~40, keep-on-failure, mark dropped), two-stage selection for large action spaces, cost ledger showing savings go negative on small inputs |
| [WindowsJev](https://github.com/Teylersf/WindowsJev) | Deterministic fast paths cost 0 tokens; Jev selects only from code-built candidates; typed text = literal span from goal; step budgets (12/20); confidence <0.80 → review; blocklists (passwords, CAPTCHA, elevation, financials, destructive, sending); one compact trace instead of screenshot loops |
| [jev-windows](https://github.com/Yzywil/jev-windows) | Freshness guards (HWND/PID binding, snapshot fingerprint, age limit, pre-act re-check, no replay after uncertain action); "proof rather than done" — `verified` only from exact local assertions; explicit data-minimization opt-in; inspectable per-category budgets |
| [jev-desktop](https://github.com/ehtan-smaltai/jev-desktop) | Clean loop: cached UIA observe → one Jev call (op + target) → small LLM only for typed text → re-read target before acting; redaction before any model call; emergency stop (hotkey + mouse corner); confirm dialogs default No; honest limits |
| [jev-windows-agent](https://github.com/VBS2004/jev-windows-agent) | Planner/executor split (frontier LLM plans, Jev executes bounded subtasks); terminal states `SUBTASK_COMPLETE/BLOCKED/NEEDS_AGENT`; caller-supplied literal inputs + per-task shortcut vocabulary validated locally; cold accessibility-tree waits; Jev via OpenRouter `~typesafe/jev-latest` as alternate route |
| [jev-experiments](https://github.com/dabit3/jev-experiments) | Where to embed Jev in system tooling: shell guard, log sentinel, launcher (NL intent → fixed command set), lint/rerank/dispatch, cost dashboard |
| [MindStudio: 12 tested use cases](https://www.mindstudio.ai/blog/jev-use-cases-automation) | Economics: ~1,000 items in ~6 s / $0.09 parallelized vs ~5 min / $0.62 on an LLM; ~20k requests <$1; pattern = Jev triages at volume, LLM handles only the flagged subset |
| [ZimaSpace: 20 open-source JEV projects](https://shop.zimaspace.com/blogs/tech-ai-hub/open-source-jev-projects) | `state → bounded decision → ordinary policy → action`; Winnow/compaction (selection instead of summarization); SemDecide (semantic predicates in shell pipelines); Canny (deterministic evidence + Jev as semantic judgment, never the permission layer); judgment above hard safety loops |
| [Vapi: early JEV experiments](https://vapi.ai/blog/early-experiment-with-jev) | Rule craft: plain and short; repeat the verb ("do not recommend a medicine, do not recommend a dose…"); false positives cost as much as misses; test rules on an eval set first; an extra layer, never a guarantee |

---

## 3. Architecture

```
host LLM (Claude/Codex/…)
        │  one MCP tool call
        ▼
sys-mcp (Rust)
  ├─ deterministic fast paths ──────────────► 0 Jev tokens (exact rules, allowlists,
  │    (regex/OS checks, permissions)         arithmetic, required fields)
  ├─ state builder ────────────────────────►  structured state from existing tools
  │    (accessibility_tree / screen_read /       or the caller's payload
  │     window_list / logs / fs / shell output)
  ├─ decision client (new) ────────────────►  POST /v1/systemone, batched questions
  ├─ policy layer (Rust) ──────────────────►  thresholds, budgets, blocklists,
  │                                            freshness re-check, redaction
  ├─ executor (existing tools) ────────────►  mouse_*/keyboard_*/shell_exec/fs_*/…
  └─ verifier (Rust) ──────────────────────►  exact local assertions → verified /
                                               uncertain → review status
        │
        ▼
compact typed result + status + confidence + usage back to the host LLM
```

### Module layout (proposed)

```
src/
  config.rs              + jev: endpoint, api_key, model, thresholds, budgets
  decision/
    mod.rs
    client.rs            HTTP client for /v1/systemone (reqwest/ureq, retries, 429+Retry-After)
    types.rs             Question/Answer serde types: noul | choice | score
    policy.rs            thresholds, confidence gates, status vocabulary
    redact.rs            secret/password/token scrubbing before any outbound state
  mcp/
    tools.rs             + decision, decision_guard tools
    handlers/
      decision.rs        new handler
```

---

## 4. Phases

### Phase 1 — Decision tool (foundation)
- [ ] Config block `jev` in config file + env override (`JEV_ENDPOINT`, `JEV_API_KEY`, `JEV_MODEL`),
      default model `jev-1.13-free`; key never appears in tool output.
- [ ] `decision/types.rs`: request `{model, state, questions}` and typed answers
      (`noul`, `choice`, `score` incl. `legend`, `probabilities`, `confidence`, `usage`).
- [ ] `decision/client.rs`: POST `/v1/systemone`, JSON body, timeout ~10 s, retry on 429 honoring
      `Retry-After`, surface upstream error envelope (`invalid_api_key` etc.) as typed errors.
- [ ] `decision/redact.rs`: scrub `password=…`, bearer keys, obvious secrets from state before send.
- [ ] MCP tool `decision`:
      `decision(state, questions[]) -> {answers, model, usage, request_id?}` where each question is
      `{id, type: noul|choice|score, instructions, criteria?}`.
- [ ] Unit tests with fixture payloads (offline) + one live integration test behind an env flag.
- [ ] Docs: README section + example curl/task.

### Phase 2 — Guardrails on destructive tools
- [ ] `decision_guard` internal helper: given an action descriptor + context, ask 1–2 `noul`
      questions (reversible? policy-violating? urgent?).
- [ ] Wrap high-risk handlers: `shell_exec` (dangerous command classes), `fs_delete`,
      `process_kill`, `service_stop`, `service_start`.
- [ ] Policy: confidence/threshold table in config; below threshold → **ask for confirmation,
      never auto-allow and never silently auto-deny**; hard denylist stays pure Rust and applies
      *before* any model call.
- [ ] Opt-in/out flag (`jev.guardrails = true|false`) and per-tool override.
- [ ] Eval set: 30–50 representative risky/benign actions with expected labels; record
      false-positive/false-negative rates before enabling by default (Vapi lesson).

### Phase 3 — State helpers & token efficiency ✅ shipped
- [x] `decision_rank` capability: split large text (logs, file contents, shell output) into blocks →
      Jev rates blocks in batches (~40) against a question → return top-k in original order,
      **keep blocks on failure, mark dropped blocks explicitly** (never silently cut).
- [x] Floor: skip Jev entirely for small inputs (below ~8K chars) where latency/cost exceeds saving.
- Deferred: `screen_read`/`accessibility_tree` normalisation into numbered `choice` candidates —
  not needed yet; OCR output is already structured and budgeted.

### Phase 4 — Bounded action loop (optional, higher risk)
- [ ] `ui_decide` tool: observe foreground window via existing `accessibility_tree` → build bounded
      candidate action list (invoke/click/type-literal/select/toggle + target) → one Jev `choice`
      (+ completion `noul`) → freshness re-check (target still present, same name/position) →
      execute via existing input tools → independent assertion → status:
      `done | likely_done | needs_confirmation | error | stuck | ambiguous | blocked`.
- [ ] Safety: step budget (default 12, max 20), irreversible-action confirmation gate,
      typed text must be a literal span supplied by the caller, foreground-window scoping,
      emergency stop hook, never drive an elevated window (PermissionError → `blocked`).
- [ ] Explicitly out of scope for now: OCR fallback for canvas apps, drag-drop, cross-window flows.

### Phase 5 — Observability & cost ledger ✅ shipped
- [x] Log per decision: model, tag, tokens, latency, success/failure (redacted).
- [x] Track `usage.input_tokens/output_tokens`, call counts, latency percentiles (min/p50/p95/max/mean),
      broken down by caller tag; exposed via the `decision_stats` tool with an optional reset.
- Deferred: the caller-tokens-saved vs Jev-tokens-spent comparison dashboard. The ledger reports spend
      plainly; the "saved" side needs real traffic to measure against, so it is not invented.

---

## 5. Safety & policy rules (non-negotiable)

1. Jev is a **signal**, never an authority: allowlists, denylists, permissions, arithmetic,
   required fields, and idempotency stay in Rust.
2. **Redact before send**: API keys, `password=` values, tokens; minimize state (labels/goals,
   not full dumps) and document that minimization ≠ anonymization.
3. **No free text from the model**: Jev only picks from offered options; typed text comes from
   caller-supplied literal values (or a small local/aux LLM for `keyboard_type`).
4. **Verify independently**: `verified` only from exact local assertions; the model's "done" is
   judgment and is reported as `likely_done` until checked.
5. **Budgets**: max actions per goal, max Jev calls per request, per-day token budget, and
   explicit stop on repeated failure (no blind retry loops).
6. **Fail explicitly**: timeouts, 429s, and low confidence map to visible statuses with a defined
   fail-open/fail-closed choice per tool (review-queue tools may fail open; destructive tools fail closed).
7. **Never claim guarantees**: "works in every circumstance" is not a truthful statement — record
   failure states (inaccessible UI, auth, network) instead of silently completing.

---

## 6. Testing strategy

| Layer | Approach |
|---|---|
| Types/serialization | Offline fixture tests for request/response shapes (all three question types, error envelope) |
| Client | Wiremock-style tests: 200, 400 validation, 401, 429+Retry-After, timeout |
| Policy | Threshold table unit tests incl. boundary values; denylist precedence tests |
| Redaction | Corpus of secrets/passwords that must never leave the process |
| Eval set | Labeled action/state corpus → accuracy, calibration, FP/FN cost per threshold (replay, no side effects) |
| Live | Feature-flagged integration test against the local gateway (`jev_body`-style payloads) |
| Regression | Re-run eval set before any model/prompt/threshold change |

---

## 7. Risks & open questions

- **Rate limits**: free model = 1 req/min (10 after top-up) upstream — guardrails on hot paths must
  cache/skip when throttled; confirm the local gateway's limits.
- **Latency**: ~300–500 ms per call is fine inside a tool call, but a guard on every `shell_exec`
  adds up; measure before enabling broadly.
- **Confidence ≠ correctness**: thresholds must come from our own eval set, not copied numbers.
- **Endpoint/key ownership**: is `192.168.2.49:5050` the permanent gateway, or do we point at
  `api.beatapi.io` / TypeSafe directly? Config must support both.
- **Scope check**: do we want Phase 4 (bounded UI loop) inside sys-mcp, or keep sys-mcp as
  primitives + let a host agent own the loop?
- **Cost attribution**: decide whether usage reporting feeds into `system_stats` or a dedicated tool.

---

## 8. Success criteria

- `decision` tool answers noul/choice/score correctly against the gateway with typed, validated output.
- Destructive tools sit behind a configurable guard with measured FP/FN rates on the eval set.
- Host-LLM token usage for log/file triage drops measurably (target: ≥5× on large inputs) with
  explicit accounting of Jev tokens spent.
- Zero secrets observed in outbound state (redaction test corpus passes).
- All statuses, budgets, and failure modes visible in tool output — no silent completion claims.

---

## 9. Reference links

- TypeSafe docs: https://docs.typesafe.ai · blog: https://typesafe.ai/blog/introducing-system-one-models-and-jev
- BeatAPI: https://beatapi.io/jev-api · docs: https://docs.beatapi.io/decisions
- Independent guides: https://jevapi.io · https://opencode.ai/v2/docs/console/models
- Ecosystem: https://github.com/BeatAPI/awesome-jev
- Local gateway (verified): `POST http://192.168.2.49:5050/v1/systemone` · `Bearer sk-dev-key`
