# TalkServo Model Tier System

> Five-tier model mapping: premium-max / premium / fast / vision / lite
> Dual-provider architecture: DeepSeek official direct + New API gateway aggregation

## Architecture

TalkServo connects to LLMs through two independent paths:

| Provider | Access Method | Characteristics |
|--------|----------|------|
| **DeepSeek Official** | Direct API (`api.deepseek.com`) | Low latency, high stability, officially managed model versions |
| **New API Gateway** | Alias proxy (`192.168.100.47:3000`) | Multi-provider aggregation, automatic failover, dynamic model switching |

The New API gateway aggregates DeepSeek / Qwen / Kimi / Doubao / GLM / MiniMax providers internally, enabling multi-model failover per provider. Changing the alias mapping on the gateway side switches models globally with zero project config changes.

DeepSeek official direct connection serves as an independent fallback path when the New API gateway is unavailable, forming a complete high-availability chain.

## Five-Tier Model Mapping

### Dual-Provider Comparison Table

| Tier | Alias | Purpose | DeepSeek Official | New API Gateway | New API Fallback 1 | New API Fallback 2 |
|------|------|------|---------------|-------------|-------------------|-------------------|
| premium-max | Ultimate Reasoning | Most complex tasks | `deepseek-reasoner` | `deepseek-v4-pro-max` | `kimi-k2.6` | `minimax-m3` |
| premium | Primary | Orchestration/building/planning | `deepseek-chat` | `deepseek-v4-pro` | `qwen3.7-max` | `glm-5.1` |
| fast | Fast | Execution/search/review | `deepseek-coder` | `deepseek-v4-flash` | `qwen3.6-flash` | `doubao-seed-2.0-lite` |
| vision | Vision | Multimodal analysis | `deepseek-vl2` | `doubao-seed-2.0-pro` | `qwen3.6-plus` | — |
| lite | Lightweight | Minimal/trivial tasks | `deepseek-chat` | `qwen3-32b` | `qwen3-8b` | — |

### New API Alias Mapping Notes

The New API gateway references models through semantic aliases (`premium` / `fast` etc.), and OpenCode config references these aliases instead of specific model names. To switch models, only the alias mapping on the gateway side needs to change; no project config file edits required.

Current gateway alias mapping:

```
premium-max       → deepseek-v4-pro-max        # Flagship reasoning
premium-max-1     → kimi-k2.6                  # Fallback 1
premium-max-2     → minimax-m3                 # Fallback 2

premium           → deepseek-v4-pro            # Primary reasoning
premium-1         → qwen3.7-max                # Fallback 1
premium-2         → glm-5.1                    # Fallback 2

fast              → deepseek-v4-flash          # Fast reasoning
fast-1            → qwen3.6-flash              # Fallback 1
fast-2            → doubao-seed-2.0-lite       # Fallback 2

vision            → doubao-seed-2.0-pro        # Vision primary
vision-1          → qwen3.6-plus               # Fallback 1
vision-2          → gemini-3.5-flash           # Fallback 2

lite              → qwen3-32b                  # Lightweight tasks
lite-1            → qwen3-8b                   # Fallback
```

## Detailed Tier Descriptions

### premium-max: Ultimate Reasoning

- **Purpose**: Most complex tasks requiring deep reasoning and multi-step thinking
- **Use cases**: Architecture design, system planning, complex refactoring, technical review, security audit
- **DeepSeek Official Model**: `deepseek-reasoner` — focused on complex reasoning chains, long context
- **New API Primary**: `deepseek-v4-pro-max` / `kimi-k2.6` / `minimax-m3` — flagship model chain
- **Temperature**: 0.2 (high determinism)
- **Reasoning Effort**: high
- **Notes**: Highest latency, highest cost; use only when deep reasoning is truly needed

### premium: Primary Reasoning

- **Purpose**: Daily complex tasks requiring strong understanding and generation
- **Use cases**: Agent orchestration, build script authoring, plan generation, code review, architecture consulting
- **DeepSeek Official Model**: `deepseek-chat` — DeepSeek latest chat model with strong general capabilities
- **New API Primary**: `deepseek-v4-pro` / `qwen3.7-max` / `glm-5.1` — primary reasoning chain
- **Temperature**: 0.2 (high determinism)
- **Notes**: Most frequently used tier in the project; balances capability and cost

### fast: Speed Execution

- **Purpose**: Simple execution tasks requiring low latency and high throughput
- **Use cases**: Code search, file exploration, simple code edits, quick review, library search
- **DeepSeek Official Model**: `deepseek-coder` — coding-optimized model, fast response
- **New API Primary**: `deepseek-v4-flash` / `qwen3.6-flash` / `doubao-seed-2.0-lite`
- **Temperature**: 0.0 (fully deterministic)
- **Notes**: Most latency-sensitive tier; does not support complex reasoning; for high-frequency small tasks

### vision: Vision Expert

- **Purpose**: Multimodal understanding, image/PDF content analysis
- **Use cases**: UI screenshot analysis, document scanning, diagram understanding, visual review
- **DeepSeek Official Model**: `deepseek-vl2` — vision-language model
- **New API Primary**: `doubao-seed-2.0-pro` / `qwen3.6-plus`
- **Temperature**: 0.1 (very high determinism)
- **Notes**: Use only when multimodal capability is needed; pure text tasks should use other tiers

### lite: Lightweight

- **Purpose**: Minimal tasks, maximum cost efficiency
- **Use cases**: Simple Q&A, log summarization, content formatting, metadata generation
- **DeepSeek Official Model**: `deepseek-chat` (lightweight invocation)
- **New API Primary**: `qwen3-32b` / `qwen3-8b`
- **Temperature**: 0.2
- **Notes**: Limited capability; use only for high-determinism tasks that don't require creativity

## Agent Tier Assignment

### Agents

| Agent | Tier | Description |
|-------|------|------|
| oracle | premium-max | Architecture consulting, technical decisions |
| sisyphus | premium | Primary coordinator agent |
| hephaestus | premium | Build management |
| prometheus | premium-max | Plan generation |
| atlas | premium | Implementation execution |
| librarian | fast | Library/doc search |
| explore | fast | Code exploration |
| metis | fast | Metrics and data analysis |
| momus | premium | Review and critique |
| sisyphus-junior | fast | Simple execution |
| multimodal-looker | vision | Visual analysis |

### Categories (task() dispatch)

| Category | Tier | Description |
|------|------|------|
| visual-engineering | premium | Visual engineering |
| ultrabrain | premium-max | Ultra-deep thinking tasks |
| deep | premium | Deep analysis |
| unspecified-high | premium | High-complexity unclassified |
| artistry | fast | Creative/docs writing |
| quick | fast | Simple tasks |
| unspecified-low | fast | Low-complexity unclassified |
| writing | fast | Documentation writing |

## Selection Guide

### When to Use Each Tier

| Task Type | Recommended Tier | Alternative |
|----------|----------|------|
| Architecture design / system review | premium-max | premium |
| Agent orchestration / planning | premium | fast (simple steps) |
| Code writing / refactoring | premium | fast (small changes) |
| File search / grep | fast | — |
| Bug debugging | premium | premium-max (hard-to-reproduce bugs) |
| UI screenshot analysis | vision | premium (no vision needed) |
| Log summarization / formatting | lite | fast |
| Code review | premium | fast (simple format review) |
| Security audit | premium-max | premium |
| Simple Q&A | lite | fast |

### Quick Decision

1. **Need deep thinking?** → premium-max
2. **Need comprehensive understanding and coding?** → premium
3. **Simple execution or search?** → fast
4. **Need to view images/PDFs?** → vision
5. **A few sentences will do?** → lite

## Provider Selection Guide

### When to Use DeepSeek Official

- When the New API gateway is unreachable (network isolation / VPN disconnected)
- Lowest latency needed (direct connection skips one gateway hop)
- DeepSeek official released a new model version not yet synced to the gateway
- DeepSeek official-only features needed (e.g., `deepseek-reasoner`'s detailed reasoning process)

### When to Use New API Gateway

- Daily development (default path)
- Multi-provider automatic failover needed (auto-switch on model timeout)
- Non-DeepSeek model capabilities needed (e.g., Doubao vision, Kimi long context)
- Gateway-side optimal model mapping already configured; no need to manage specific providers

### Priority Order

```
New API gateway → automatic failover → DeepSeek official direct
```

OpenCode's `runtime_fallback` mechanism handles the above logic automatically:
1. First try New API gateway alias (e.g., `new-api/premium`)
2. On failure, try New API Fallback models (`new-api/premium-1`, `new-api/premium-2`)
3. After all fail, manually switch config to DeepSeek official model ID

## Fallback Chain

### Agent-Level Fallback

Each agent defines a 3-level fallback in `oh-my-openagent.jsonc`:

```jsonc
"oracle": {
  "model": "new-api/premium-max",       // Primary
  "fallback_models": [
    "new-api/premium-max-1",            // Fallback 1
    "new-api/premium-max-2"             // Fallback 2
  ]
}
```

### Runtime Fallback

`runtime_fallback` global config handles network-level errors:

| Config | Value | Description |
|--------|-----|------|
| retry_on_errors | 402, 429, 500, 502, 503, 504 | HTTP status codes that trigger fallback |
| max_fallback_attempts | 2 | Maximum 2 failover attempts |
| cooldown_seconds | 60 | 60-second cooldown after failure |
| timeout_seconds | 60 | Per-request timeout |

### Fallback Data Flow

```
User request
  │
  ▼
Agent primary model (new-api/premium)
  │  ├─ Success → return result
  │  └─ Failure → retry (max 2 attempts)
  │       │
  │       ▼
  │   Fallback 1 (new-api/premium-1)
  │     ├─ Success → return result
  │     └─ Failure → retry
  │          │
  │          ▼
  │      Fallback 2 (new-api/premium-2)
  │        ├─ Success → return result
  │        └─ All failed → gateway unavailable, switch to DeepSeek official
  │             │
  │             ▼
  │         DeepSeek official (deepseek-chat / deepseek-reasoner)
  │           └─ Success/failure → return result or error
```

## Adding New Models

1. Confirm model availability on DeepSeek official, obtain API endpoint
2. Register new model channel on New API gateway and create alias mapping
3. Update the mapping table in this document
4. No OMO config changes needed (alias references are used)

## Important Notes

- DeepSeek official model IDs may change with updates; refer to `api.deepseek.com` docs
- New API alias mappings are maintained by the gateway admin; project members don't need to manage specific mappings
- When switching to DeepSeek official direct, ensure the API key is configured in environment variables
- Vision tasks prefer New API (Doubao vision outperforms DeepSeek official)
- Using the lite tier for low-cost tasks significantly reduces token consumption
