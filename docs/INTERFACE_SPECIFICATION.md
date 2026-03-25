# Interface Specification

## Unbroken Test Platform — Rust Trait Definitions

This document describes the interfaces (Rust traits) that form the contract
between components of the test platform, and the concrete implementations
behind them. All code is pure Rust with zero third-party dependencies and
is WASM-compatible.

---

## Module Overview

```
src/
├── lib.rs                Module root
│
│   Traits (interfaces)
├── types.rs              Core data structures (no behavior)
├── registry.rs           TestRegistry trait
├── filter.rs             TestFilter trait
├── executor.rs           RunnableTest + TestExecutor traits
├── progress.rs           ProgressTracker trait
├── discovery.rs          TestDiscovery trait
├── reporter.rs           TestReporter trait
├── manager.rs            TestManager trait
│
│   Infrastructure
├── json.rs               Hand-rolled JSON parser + serializer
├── json_types.rs         ToJson/FromJson for all domain types
├── storage.rs            JSON file persistence (with WASM stubs)
│
│   Concrete implementations
├── impl_registry.rs      InMemoryRegistry
├── impl_filter.rs        StandardFilter
├── impl_executor.rs      SequentialExecutor
├── impl_progress.rs      InMemoryProgressTracker
├── impl_discovery.rs     RegistryDiscovery
├── impl_reporter.rs      StandardReporter (JSON + Text)
├── impl_manager.rs       PlatformManager (top-level orchestrator)
│
│   Caller interfaces
├── console.rs            Console interface (human operators)
└── mcp.rs                MCP tool interface (AI agents)
```

---

## System Architecture

```mermaid
graph TB
    subgraph Callers["Caller Layer"]
        AI[AI Agent]
        Human[Human Operator]
    end

    subgraph Interface["Interface Layer"]
        MCP[MCP Tools<br/>mcp.rs]
        CON[Console<br/>console.rs]
    end

    subgraph Orchestration["Orchestration Layer"]
        MGR[PlatformManager<br/>impl_manager.rs]
    end

    subgraph Core["Core Layer"]
        REG[InMemoryRegistry<br/>impl_registry.rs]
        FILT[StandardFilter<br/>impl_filter.rs]
        EXEC[SequentialExecutor<br/>impl_executor.rs]
        PROG[InMemoryProgressTracker<br/>impl_progress.rs]
        DISC[RegistryDiscovery<br/>impl_discovery.rs]
        RPT[StandardReporter<br/>impl_reporter.rs]
    end

    subgraph Infrastructure["Infrastructure Layer"]
        JSON[JSON Module<br/>json.rs + json_types.rs]
        STORE[Storage<br/>storage.rs]
    end

    AI --> MCP
    Human --> CON
    MCP --> MGR
    CON --> MGR
    MGR --> REG
    MGR --> FILT
    MGR --> EXEC
    MGR --> PROG
    MGR --> DISC
    MGR --> RPT
    REG --> JSON
    STORE --> JSON
    MGR --> STORE
```

---

## Interface Dependency Diagram (Traits)

```mermaid
graph TD
    MGR[TestManager]
    DISC[TestDiscovery]
    REG[TestRegistry]
    FILT[TestFilter]
    EXEC[TestExecutor]
    RUN[RunnableTest]
    PROG[ProgressTracker]
    RPT[TestReporter]

    MGR --> DISC
    MGR --> REG
    MGR --> FILT
    MGR --> EXEC
    MGR --> PROG
    MGR --> RPT
    DISC --> REG
    FILT --> REG
    EXEC --> RUN
    EXEC --> PROG
```

---

## Data Flow

```mermaid
flowchart LR
    subgraph Input
        JSON[JSON Request]
    end

    subgraph Interfaces
        MCP[MCP Tool]
        CON[Console]
    end

    subgraph Manager
        M[PlatformManager]
    end

    subgraph Pipeline
        direction LR
        D[Discovery] --> F[Filter] --> E[Executor] --> C[Result Collector]
    end

    subgraph Output
        direction TB
        PROG[Progress Snapshots]
        SUM[RunSummary JSON]
        TEXT[Text Report]
    end

    JSON --> MCP
    JSON --> CON
    MCP --> M
    CON --> M
    M --> D
    E -.->|real-time| PROG
    C --> SUM
    C --> TEXT
    SUM --> MCP
    TEXT --> CON
```

---

## Traits

### `TestRegistry` — `src/registry.rs`

Source of truth for all known tests.

| Method | Signature | Purpose |
|---|---|---|
| `register` | `(&mut self, TestDefinition) -> Result<(), RegistryError>` | Add a test |
| `deregister` | `(&mut self, &str) -> Option<TestDefinition>` | Remove a test |
| `get` | `(&self, &str) -> Option<&TestDefinition>` | Lookup by ID |
| `list_all` | `(&self) -> Vec<&TestDefinition>` | All tests |
| `count` | `(&self) -> usize` | Total count |
| `search_by_name` | `(&self, &str) -> Vec<&TestDefinition>` | Name search |
| `filter_by_tags` | `(&self, &[String]) -> Vec<&TestDefinition>` | Tag filter |
| `filter_by_group` | `(&self, &str) -> Vec<&TestDefinition>` | Group filter |
| `all_tags` | `(&self) -> Vec<String>` | Known tags |
| `all_groups` | `(&self) -> Vec<String>` | Known groups |

**Implementation**: `InMemoryRegistry` — Vec-backed, JSON serializable.

### `TestFilter` — `src/filter.rs`

Applies RunConfig criteria to produce the execution subset.

| Method | Signature | Purpose |
|---|---|---|
| `apply` | `(&self, &[&TestDefinition], &RunConfig) -> Vec<&TestDefinition>` | Filter tests |

Filter precedence: include IDs → include tags → name pattern → exclude tags.

**Implementation**: `StandardFilter` — supports glob patterns, tag intersection, exclusion.

### `RunnableTest` — `src/executor.rs`

Wraps actual test logic. Each test in the system implements this.

| Method | Signature | Purpose |
|---|---|---|
| `id` | `(&self) -> &str` | Test identity |
| `run` | `(&self, Option<DurationMs>) -> TestResult` | Execute the test |

### `TestExecutor` — `src/executor.rs`

Runs a batch of tests and reports results as they complete.

| Method | Signature | Purpose |
|---|---|---|
| `execute` | `(&self, &[&dyn RunnableTest], ...) -> Vec<TestResult>` | Run batch |

Supports `fail_fast` and per-test `on_result` callback for progress.

**Implementation**: `SequentialExecutor` — runs tests one at a time, skips remaining on fail_fast.

### `ProgressTracker` — `src/progress.rs`

Real-time visibility into running suites.

| Method | Signature | Purpose |
|---|---|---|
| `start_run` | `(&mut self, RunId, u32)` | Begin tracking |
| `test_started` | `(&mut self, &str, &str)` | Mark test as running |
| `test_completed` | `(&mut self, &str, &TestResult)` | Record result |
| `get_progress` | `(&self, &str) -> Option<RunProgress>` | Snapshot |
| `finish_run` | `(&mut self, &str)` | Mark complete |
| `active_runs` | `(&self) -> Vec<RunId>` | List in-flight runs |

**Implementation**: `InMemoryProgressTracker` — pluggable clock, JSON serializable.

### `TestDiscovery` — `src/discovery.rs`

Caller-facing search and explore API.

| Method | Signature | Purpose |
|---|---|---|
| `discover` | `(&self, &DiscoveryQuery) -> DiscoveryResult` | Search tests |
| `summary` | `(&self) -> DiscoverySummary` | Overview stats |

**Implementation**: `RegistryDiscovery` — delegates to registry, supports pagination.

### `TestReporter` — `src/reporter.rs`

Formats output for delivery.

| Method | Signature | Purpose |
|---|---|---|
| `format_summary` | `(&self, &RunSummary, ReportFormat) -> String` | Format results |
| `format_progress` | `(&self, &RunProgress, ReportFormat) -> String` | Format progress |

Supports `Json` and `Text` output formats.

**Implementation**: `StandardReporter` — JSON for AI, text with progress bars for humans.

### `TestManager` — `src/manager.rs`

Top-level orchestrator. Both MCP and console interfaces talk to this.

| Method | Signature | Purpose |
|---|---|---|
| `discover` | `(&self, &DiscoveryQuery) -> DiscoveryResult` | Search tests |
| `summary` | `(&self) -> DiscoverySummary` | Overview |
| `register_test` | `(&mut self, TestDefinition) -> Result<(), ManagerError>` | Add test |
| `start_run` | `(&mut self, RunConfig) -> Result<RunId, ManagerError>` | Kick off run |
| `check_progress` | `(&self, &str) -> Result<RunProgress, ManagerError>` | Check in |
| `active_runs` | `(&self) -> Vec<RunId>` | List running |
| `get_results` | `(&self, &str) -> Result<RunSummary, ManagerError>` | Final results |

**Implementation**: `PlatformManager` — wires all components, persists to JSON storage.

---

## MCP Tools — `src/mcp.rs`

The AI-facing interface. Each tool accepts JSON params and returns a JSON response.

```mermaid
flowchart TD
    AI[AI Agent] -->|JSON request| PARSE[parse_request]
    PARSE --> DISPATCH[handle_request]
    DISPATCH --> TL[tool_list]
    DISPATCH --> TS[test_summary]
    DISPATCH --> TD[test_discover]
    DISPATCH --> TR[test_run]
    DISPATCH --> TP[test_progress]
    DISPATCH --> TRS[test_results]
    DISPATCH --> TLT[test_list_tags]
    DISPATCH --> TLG[test_list_groups]
    TL --> RESP[McpResponse JSON]
    TS --> RESP
    TD --> RESP
    TR --> RESP
    TP --> RESP
    TRS --> RESP
    TLT --> RESP
    TLG --> RESP
    RESP -->|JSON response| AI
```

| Tool | Parameters | Returns |
|---|---|---|
| `tool_list` | — | Array of tool descriptors with parameter schemas |
| `test_summary` | — | Total tests, tags with counts, groups with counts |
| `test_discover` | `name_pattern`, `tags`, `group`, `limit`, `offset` | Matching tests, total count, available tags/groups |
| `test_run` | `run_all`, `include_ids`, `include_tags`, `exclude_tags`, `name_pattern`, `fail_fast`, `timeout_ms` | RunSummary with per-test results |
| `test_progress` | `run_id` (optional) | RunProgress or list of active runs |
| `test_results` | `run_id` | RunSummary |
| `test_list_tags` | — | Tags with counts |
| `test_list_groups` | — | Groups with counts |

---

## Console Commands — `src/console.rs`

The human-facing interface. Text commands in, text + JSON out.

```mermaid
flowchart TD
    Human[Human Operator] -->|text command| PARSE[split_args + match]
    PARSE --> HELP[help]
    PARSE --> SUM[summary]
    PARSE --> DISC[discover]
    PARSE --> RUN[run]
    PARSE --> PROG[progress]
    PARSE --> RES[results]
    PARSE --> TAGS[tags]
    PARSE --> GRP[groups]
    HELP --> OUT[ConsoleOutput]
    SUM --> OUT
    DISC --> OUT
    RUN --> OUT
    PROG --> OUT
    RES --> OUT
    TAGS --> OUT
    GRP --> OUT
    OUT -->|text + json| Human
```

Every command returns `ConsoleOutput { text, json }` — text for the terminal,
JSON for debugging/storage.

---

## Caller Interaction Sequence

```mermaid
sequenceDiagram
    participant Caller as Caller (AI / Human)
    participant IF as Interface (MCP / Console)
    participant MGR as PlatformManager
    participant REG as InMemoryRegistry
    participant FILT as StandardFilter
    participant EXEC as SequentialExecutor
    participant PROG as ProgressTracker
    participant STORE as JSON Storage

    Caller->>IF: discover(query)
    IF->>MGR: discover(query)
    MGR->>REG: search/filter
    REG-->>MGR: matching tests
    MGR-->>IF: DiscoveryResult
    IF-->>Caller: JSON / Text

    Caller->>IF: start_run(config)
    IF->>MGR: start_run(config)
    MGR->>FILT: apply(all_tests, config)
    FILT-->>MGR: filtered subset
    MGR->>PROG: start_run(run_id, count)
    MGR->>EXEC: execute(tests)

    loop Each test completes
        EXEC->>PROG: test_completed(result)
    end

    par While running
        Caller->>IF: check_progress(run_id)
        IF->>MGR: check_progress(run_id)
        MGR->>PROG: get_progress(run_id)
        PROG-->>MGR: RunProgress
        MGR-->>IF: progress snapshot
        IF-->>Caller: JSON / Text
    end

    EXEC-->>MGR: all results
    MGR->>PROG: finish_run(run_id)
    MGR->>STORE: save RunSummary JSON
    Caller->>IF: get_results(run_id)
    IF->>MGR: get_results(run_id)
    MGR-->>IF: RunSummary
    IF-->>Caller: JSON / Text
```

---

## JSON Storage Layout

All platform state persists as human-readable JSON files:

```
<storage_dir>/
├── registry.json              All registered test definitions
└── runs/
    ├── run_0001.json          Results of first run
    ├── run_0002.json          Results of second run
    └── ...
```

---

## Test Coverage

72 tests across all modules:

| Module | Tests | Coverage |
|---|---|---|
| `json` | 4 | Parse, serialize, round-trip, escaping |
| `impl_registry` | 5 | Register, deregister, search, filter, JSON round-trip |
| `impl_filter` | 4 | Run all, include by ID, exclude by tag, name pattern |
| `impl_executor` | 2 | Sequential execution, fail_fast skip |
| `impl_progress` | 2 | Progress tracking, finish/active |
| `impl_discovery` | 4 | Discover all, by group, pagination, summary |
| `impl_reporter` | 2 | Text format, JSON format |
| `impl_manager` | 2 | Full lifecycle, filtered run |
| `storage` | 1 | Path formatting |
| `console` | 20 | All commands, error cases, output formats |
| `mcp` | 26 | All tools, error handling, full AI workflow |
