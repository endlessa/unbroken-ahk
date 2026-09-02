//! Drives the real console interface end to end and prints the session —
//! used to demo/screenshot the platform (`cargo run --example console_demo`).

use unbroken_test_platform::console::execute_command;
use unbroken_test_platform::impl_manager::PlatformManager;
use unbroken_test_platform::executor::RunnableTest;
use unbroken_test_platform::types::{DurationMs, TestDefinition, TestResult, TestStatus};

struct DemoTest {
    id: &'static str,
    pass: bool,
    ms: u64,
}

impl RunnableTest for DemoTest {
    fn id(&self) -> &str {
        self.id
    }
    fn run(&self, _timeout: Option<DurationMs>) -> TestResult {
        TestResult {
            test_id: self.id.into(),
            status: if self.pass { TestStatus::Passed } else { TestStatus::Failed },
            duration_ms: self.ms,
            message: if self.pass { None } else { Some("expected 200 OK, got 503".into()) },
            stdout: None,
            stderr: None,
        }
    }
}

fn def(id: &str, name: &str, tags: &[&str], group: &str) -> TestDefinition {
    TestDefinition {
        id: id.into(),
        name: name.into(),
        tags: tags.iter().map(|s| s.to_string()).collect(),
        group: Some(group.into()),
        description: None,
        metadata: vec![],
    }
}

fn main() {
    let dir = std::env::temp_dir().join("unbroken-demo");
    let _ = std::fs::remove_dir_all(&dir);
    let mut mgr = PlatformManager::new(dir.to_str().unwrap());

    for (id, name, tags, group, pass, ms) in [
        ("t1", "auth_basic_login", &["smoke", "fast"][..], "auth", true, 12),
        ("t2", "auth_token_refresh", &["smoke"][..], "auth", true, 48),
        ("t3", "net_ping_gateway", &["smoke", "fast"][..], "network", true, 7),
        ("t4", "net_dns_resolve", &["slow"][..], "network", false, 812),
        ("t5", "storage_atomic_write", &["fast"][..], "storage", true, 33),
    ] {
        mgr.register_runnable(def(id, name, tags, group), Box::new(DemoTest { id, pass, ms }))
            .unwrap();
    }

    for cmd in [
        "summary",
        "discover --tag smoke",
        "run --tag smoke --fail-fast",
        "run --exclude slow",
        "run --tag slow",
        "progress run_0003",
        "run --tag smokee",
    ] {
        println!("unbroken> {}", cmd);
        println!("{}", execute_command(&mut mgr, cmd).text.trim_end());
        println!();
    }

    let _ = std::fs::remove_dir_all(&dir);
}
