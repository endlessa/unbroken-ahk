# Project Statement

## Unbroken Test Platform

### Overview

The Unbroken Test Platform is a test orchestration system designed to serve as a
core component within a larger pure-Rust, zero-dependency ecosystem. The platform
enables both AI agents and human operators to discover, configure, execute, and
receive results from a large and growing suite of tests — currently numbering in
the hundreds.

The system runs entirely within edge WASM containers and maintains a strict
zero-third-party-dependency policy to ensure a clean, IP-free licensing posture
across the entire stack.

### Problem

As the test suite scales, there is no unified mechanism to:

- Discover what tests are available
- Search and filter tests by criteria
- Trigger full or partial test runs via structured input
- Track progress of in-flight test executions
- Package and return results to the requesting entity in a closed feedback loop

### Solution

A lightweight test orchestration platform providing:

1. **Test Discovery** — A searchable registry of all available tests, queryable
   by AI or human callers.
2. **Test Execution** — Accept JSON-based run configurations specifying full
   suite or selected subsets. Execute and manage test runs.
3. **Progress Tracking** — Real-time progress state available for check-in
   during execution, enabling callers to monitor how far along a run is.
4. **Result Packaging** — Collect output from completed runs, package it, and
   return it to the requesting caller.
5. **Dual Interface** — An MCP tool interface for AI agents and a console
   interface for human operators.

### Constraints

| Constraint | Detail |
|---|---|
| Language | Rust (pure, no exceptions) |
| Dependencies | Zero third-party crates |
| Runtime | Edge WASM containers |
| Licensing | No IP encumbrance — fully clean implementation |
| Scope | Component of a larger system; interfaces must be well-defined |

### SDLC Approach

This project follows a full Software Development Lifecycle workflow. All phases
— requirements gathering, functional specification, design, implementation,
testing, and deployment — will be documented and tracked as if operating within
a multi-person engineering department.

### Current Status

| Phase | Status |
|---|---|
| Requirements | Complete |
| General Needs Analysis | Complete |
| Interface Specification | Complete |
| Implementation — Core Types | Complete |
| Implementation — JSON Module | Complete (hand-rolled, zero deps) |
| Implementation — Registry | Complete (InMemoryRegistry) |
| Implementation — Filter | Complete (StandardFilter) |
| Implementation — Executor | Complete (SequentialExecutor) |
| Implementation — Progress | Complete (InMemoryProgressTracker) |
| Implementation — Discovery | Complete (RegistryDiscovery) |
| Implementation — Reporter | Complete (StandardReporter: JSON + Text) |
| Implementation — Manager | Complete (PlatformManager) |
| Implementation — Storage | Complete (JSON file persistence) |
| Implementation — Console | Complete (human interface) |
| Implementation — MCP | Complete (AI agent interface) |
| Testing | 72 tests passing |
| Functional Specification | Pending |
| Deployment (WASM) | Pending |

---

*This is a living document. It will evolve as requirements are refined.*
