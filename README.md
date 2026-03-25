# Unbroken Test Platform

Pure Rust, zero-dependency test orchestration platform for edge WASM containers.

## What It Does

Enables AI agents and human operators to discover, configure, execute, and
collect results from a large test suite through a unified platform.

## Interfaces

- **MCP Tools** — JSON request/response for AI agents
- **Console** — Text commands for human operators

Both interfaces produce JSON output for storage and debugging.

## Quick Reference

### Console Commands

```
summary                        Overview of all registered tests
discover                       List all tests
discover <pattern>             Search by name pattern
discover --tag <tag>           Filter by tag
discover --group <group>       Filter by group
tags                           List all tags
groups                         List all groups
run                            Run all tests
run --tag <tag>                Run by tag
run --id <id1> <id2>           Run specific tests
run --fail-fast                Stop on first failure
progress <run_id>              Check progress mid-run
results <run_id>               Get completed results
```

### MCP Tools

| Tool | Description |
|---|---|
| `tool_list` | List available MCP tools and parameter schemas |
| `test_summary` | Overview: total tests, tags, groups |
| `test_discover` | Search/filter tests by name, tag, group |
| `test_run` | Execute tests with configuration |
| `test_progress` | Check progress of a running suite |
| `test_results` | Get completed run results |
| `test_list_tags` | List all tags with counts |
| `test_list_groups` | List all groups with counts |

### MCP Example

```json
{"tool": "test_run", "params": {"run_all": false, "include_tags": ["smoke"]}}
```

## Architecture

```
src/
├── types.rs              Core data structures
├── registry.rs           TestRegistry trait
├── filter.rs             TestFilter trait
├── executor.rs           RunnableTest + TestExecutor traits
├── progress.rs           ProgressTracker trait
├── discovery.rs          TestDiscovery trait
├── reporter.rs           TestReporter trait
├── manager.rs            TestManager trait
├── json.rs               Hand-rolled JSON parser/serializer
├── json_types.rs         ToJson/FromJson for all types
├── storage.rs            JSON file persistence
├── impl_registry.rs      InMemoryRegistry
├── impl_filter.rs        StandardFilter
├── impl_executor.rs      SequentialExecutor
├── impl_progress.rs      InMemoryProgressTracker
├── impl_discovery.rs     RegistryDiscovery
├── impl_reporter.rs      StandardReporter (JSON + Text)
├── impl_manager.rs       PlatformManager (orchestrator)
├── console.rs            Console interface (human)
└── mcp.rs                MCP tool interface (AI)
```

## Constraints

- **Language**: Pure Rust, no exceptions
- **Dependencies**: Zero third-party crates
- **Runtime**: Edge WASM containers
- **Licensing**: No IP encumbrance
- **Storage**: All JSON, all the time

## Tests

72 tests covering every layer end-to-end.

```
cargo test
```

## Documentation

- [Project Statement](docs/PROJECT_STATEMENT.md)
- [Interface Specification](docs/INTERFACE_SPECIFICATION.md)
- [Test System Analysis](docs/TEST_SYSTEM_ANALYSIS.md)
