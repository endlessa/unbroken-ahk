// -- Traits (interfaces) --
pub mod types;
pub mod registry;
pub mod filter;
pub mod executor;
pub mod progress;
pub mod manager;
pub mod discovery;
pub mod reporter;

// -- Infrastructure --
pub mod json;
pub mod json_types;
pub mod storage;

// -- Concrete implementations --
pub mod impl_registry;
pub mod impl_filter;
pub mod impl_executor;
pub mod impl_progress;
pub mod impl_discovery;
pub mod impl_reporter;
pub mod impl_manager;
pub mod console;
pub mod mcp;
