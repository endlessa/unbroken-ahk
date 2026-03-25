# Test System General Needs Analysis

This document captures the common patterns found across test orchestration systems
(JUnit, pytest, cargo test, GoogleTest, vstest, etc.) to inform the design of the
Unbroken Test Platform.

---

## Core Components

Every test system has these fundamental pieces:

| Component | Responsibility |
|---|---|
| Test Registry | Knows what tests exist, holds metadata |
| Test Discovery | Scans and populates the registry |
| Test Filter | Selects a subset based on criteria |
| Test Runner/Manager | Orchestrates the full lifecycle |
| Test Executor | Actually runs individual tests |
| Progress Tracker | Tracks state of an in-flight run |
| Result Collector | Aggregates outcomes from execution |
| Reporter | Formats and delivers results to the caller |

---

## Layered Architecture

```mermaid
graph TB
    subgraph Presentation["Presentation Layer"]
        MCP[MCP Tool Interface]
        CLI[Console Interface]
    end

    subgraph Orchestration["Orchestration Layer"]
        MGR[Test Manager]
        CFG[Configuration]
        PROG[Progress Tracker]
    end

    subgraph Core["Core Layer"]
        DISC[Discovery]
        REG[Test Registry]
        FILT[Filter Engine]
        EXEC[Executor]
        COLLECT[Result Collector]
    end

    subgraph Runtime["Runtime Layer"]
        WASM[WASM Container]
        TEST1[Test A]
        TEST2[Test B]
        TEST3[Test N...]
    end

    MCP --> MGR
    CLI --> MGR
    MGR --> CFG
    MGR --> PROG
    MGR --> DISC
    MGR --> FILT
    MGR --> EXEC
    DISC --> REG
    FILT --> REG
    EXEC --> COLLECT
    EXEC --> WASM
    WASM --> TEST1
    WASM --> TEST2
    WASM --> TEST3
```

---

## Lifecycle Phases

Every test run moves through these phases regardless of framework:

```mermaid
flowchart LR
    A[Discovery] --> B[Collection]
    B --> C[Filtering]
    C --> D[Planning]
    D --> E[Execution]
    E --> F[Result Collection]
    F --> G[Reporting]

    style A fill:#2d3748,stroke:#4a5568,color:#e2e8f0
    style B fill:#2d3748,stroke:#4a5568,color:#e2e8f0
    style C fill:#2d3748,stroke:#4a5568,color:#e2e8f0
    style D fill:#2d3748,stroke:#4a5568,color:#e2e8f0
    style E fill:#2d3748,stroke:#4a5568,color:#e2e8f0
    style F fill:#2d3748,stroke:#4a5568,color:#e2e8f0
    style G fill:#2d3748,stroke:#4a5568,color:#e2e8f0
```

| Phase | What Happens |
|---|---|
| Discovery | Scan for available tests, build the registry |
| Collection | Aggregate tests into executable units, resolve metadata |
| Filtering | Apply include/exclude criteria from the run configuration |
| Planning | Determine execution order, parallelism, grouping |
| Execution | Run tests, capture output, handle timeouts |
| Result Collection | Aggregate pass/fail/skip/error outcomes and metrics |
| Reporting | Format and deliver results to the requesting caller |

---

## Core Abstractions

These are the data structures every test system defines in some form:

```mermaid
classDiagram
    class TestDefinition {
        +String id
        +String name
        +Vec~String~ tags
        +Option~String~ group
        +Map~String String~ metadata
    }

    class TestRegistry {
        +Vec~TestDefinition~ tests
        +register(test)
        +search(query) Vec~TestDefinition~
        +list_all() Vec~TestDefinition~
        +filter(criteria) Vec~TestDefinition~
    }

    class RunConfig {
        +Option~Vec~String~~ include_ids
        +Option~Vec~String~~ include_tags
        +Option~Vec~String~~ exclude_tags
        +Option~String~ name_pattern
        +ExecutionModel execution_model
        +Option~u64~ timeout_ms
        +bool fail_fast
    }

    class TestResult {
        +String test_id
        +Status status
        +u64 duration_ms
        +Option~String~ message
        +Option~String~ stdout
        +Option~String~ stderr
    }

    class RunProgress {
        +String run_id
        +u32 total
        +u32 completed
        +u32 passed
        +u32 failed
        +u32 skipped
        +u32 running
        +f64 percent_complete
    }

    class RunSummary {
        +String run_id
        +Vec~TestResult~ results
        +u32 total
        +u32 passed
        +u32 failed
        +u32 skipped
        +u64 total_duration_ms
    }

    TestRegistry --> TestDefinition
    RunSummary --> TestResult
```

---

## Execution Models

Test systems support different execution strategies:

```mermaid
flowchart TD
    subgraph Sequential
        S1[Test 1] --> S2[Test 2] --> S3[Test 3] --> S4[Test 4]
    end

    subgraph Parallel
        direction LR
        P1[Test 1]
        P2[Test 2]
        P3[Test 3]
        P4[Test 4]
    end

    subgraph Grouped["Suite-Grouped"]
        direction TB
        subgraph G1["Suite A (parallel)"]
            GA1[Test 1]
            GA2[Test 2]
        end
        subgraph G2["Suite B (sequential)"]
            GB1[Test 3] --> GB2[Test 4]
        end
    end
```

For the Unbroken platform, key decisions:
- Tests can run in the manager's WASM container (in-process)
- Tests can spawn into their own WASM containers (isolated)
- Both models may coexist depending on test needs

---

## Progress Tracking

```mermaid
sequenceDiagram
    participant Caller as Caller (AI/Human)
    participant MGR as Test Manager
    participant EXEC as Executor
    participant PROG as Progress Tracker

    Caller->>MGR: Start run (JSON config)
    MGR->>PROG: Initialize (run_id, total_count)
    MGR->>EXEC: Execute test batch

    loop For each test
        EXEC->>PROG: Update (test started)
        EXEC->>EXEC: Run test
        EXEC->>PROG: Update (test completed, result)
    end

    par During execution
        Caller->>MGR: Check progress (run_id)
        MGR->>PROG: Get current state
        PROG-->>MGR: RunProgress
        MGR-->>Caller: Progress snapshot (JSON)
    end

    EXEC-->>MGR: All tests complete
    MGR->>Caller: RunSummary (JSON)
```

---

## Discovery and Search

```mermaid
flowchart TD
    A[Caller] -->|"list all"| B[Discovery API]
    A -->|"search by name"| B
    A -->|"filter by tags"| B
    A -->|"filter by group"| B
    B --> C[Test Registry]
    C --> D[Return matching tests as JSON]
    D --> A
```

Discovery is a prerequisite step — callers query the registry before deciding
what to run. The registry must support:

- **List all** — enumerate every registered test
- **Search by name** — substring or pattern match on test names
- **Filter by tags** — include/exclude based on tag sets
- **Filter by group** — select tests belonging to a logical group

---

## Result Status

```mermaid
stateDiagram-v2
    [*] --> Pending: Test queued
    Pending --> Running: Executor picks up
    Running --> Passed: Assertions pass
    Running --> Failed: Assertion failure
    Running --> Error: Unexpected crash/timeout
    Running --> Skipped: Skip condition met
    Passed --> [*]
    Failed --> [*]
    Error --> [*]
    Skipped --> [*]
```

---

## Configuration Input (JSON)

A run request would look something like:

```json
{
  "run_all": false,
  "include_ids": ["test_auth_basic", "test_auth_token"],
  "include_tags": ["smoke"],
  "exclude_tags": ["slow"],
  "name_pattern": "auth_*",
  "fail_fast": true,
  "timeout_ms": 30000
}
```

When `run_all` is true, filters are ignored and the full suite executes.

---

## Summary of General Needs

1. **Registry** — A single source of truth for what tests exist
2. **Discovery/Search** — Query the registry before running
3. **Flexible Filtering** — By ID, name, tag, group, pattern
4. **Configurable Execution** — Sequential, parallel, or grouped
5. **Progress Visibility** — Real-time check-in during runs
6. **Structured Results** — Machine-readable output (JSON) with status, timing, output capture
7. **Dual Interface** — Same core, different front-ends (MCP for AI, console for humans)
8. **Isolation** — Tests should not affect each other

---

*Next step: Map these general needs onto the specific Unbroken Test Platform
architecture, considering pure Rust, zero dependencies, and edge WASM constraints.*
