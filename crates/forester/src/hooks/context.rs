//! Hook execution context.

use super::Trigger;
use crate::domain::ForestRoot;
use crate::events::{EventEmitter, ForesterEvent};
use bon::Builder;
use std::path::PathBuf;
use std::sync::Arc;

/// Context passed to hooks during execution.
#[derive(Clone, Builder)]
#[non_exhaustive]
pub struct HookContext {
    /// Root directory of the forest.
    pub forest_root: ForestRoot,
    /// Current trigger.
    pub trigger: Trigger,
    /// Event emitter for hook output.
    pub emitter: Arc<dyn EventEmitter>,
    /// Name of the grove being operated on.
    #[builder(into)]
    pub grove_name: Option<String>,
    /// Path to the grove being operated on.
    #[builder(into)]
    pub grove_path: Option<PathBuf>,
    /// Whether verbose output is enabled.
    #[builder(default)]
    pub verbose: bool,
}

impl HookContext {
    /// Emits an event through the context's emitter.
    pub fn emit(&self, event: &ForesterEvent) {
        self.emitter.emit(event);
    }

    /// Returns environment variables for hook execution.
    pub fn env_vars(&self) -> Vec<(&'static str, String)> {
        let mut vars = vec![
            ("FOREST_ROOT", self.forest_root.path().display().to_string()),
            ("TRIGGER", self.trigger.to_string()),
        ];
        if let Some(name) = &self.grove_name {
            vars.push(("GROVE_NAME", name.clone()));
        }
        if let Some(path) = &self.grove_path {
            vars.push(("GROVE_PATH", path.display().to_string()));
        }
        vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct TestEmitter {
        events: Mutex<Vec<ForesterEvent>>,
    }

    impl TestEmitter {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<ForesterEvent> {
            self.events.lock().unwrap().clone()
        }
    }

    impl EventEmitter for TestEmitter {
        fn emit(&self, event: &ForesterEvent) {
            self.events.lock().unwrap().push(event.clone());
        }
    }

    #[test]
    fn hook_context_can_be_built_with_emitter() {
        let emitter = Arc::new(TestEmitter::new());
        let ctx = HookContext::builder()
            .forest_root(ForestRoot::builder().path("/tmp/forest").build())
            .trigger(Trigger::PostGroveCreate)
            .emitter(emitter as Arc<dyn EventEmitter>)
            .build();
        assert!(ctx.grove_name.is_none());
    }

    #[test]
    fn emit_delegates_to_emitter() {
        let emitter = Arc::new(TestEmitter::new());
        let ctx = HookContext::builder()
            .forest_root(ForestRoot::builder().path("/tmp/forest").build())
            .trigger(Trigger::PostGroveCreate)
            .emitter(emitter.clone() as Arc<dyn EventEmitter>)
            .build();

        ctx.emit(&ForesterEvent::Info("test".to_string()));

        let events = emitter.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ForesterEvent::Info(ref s) if s == "test"));
    }
}
