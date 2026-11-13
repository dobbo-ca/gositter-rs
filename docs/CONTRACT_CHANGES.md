# Contract changes

Changes to the frozen interface in `src/api.rs` (and the public API it names), after it
merged. One dated entry per change: what changed, why, and which stream asked. Tell both
streams through the coordinator.

## 2026-11-13: `ExternalScanner: Any`, and the `Parser::kept_scanner` hook

- `pub trait ExternalScanner: std::any::Any + Send` (was `Send`). A scanner owns its data, so it
  is `'static`; nothing else changes for a scanner.
- `Parser::kept_scanner(&self) -> Option<&dyn ExternalScanner>` in `src/export.rs`
  (`#[doc(hidden)]`, gositter `export_test.go` `KeptScanner`). It returns the scanner the parser
  keeps for its next parse in the same language; it works once the parser lands (plan Task 11).
- Why: gositter `reset_test.go` (`TestResetIsNew` for bash, cpp, hcl, python;
  `TestResetDropsBigBuffers` for bash, hcl) inspects the kept scanner. With the supertrait, a test
  upcasts it (`let a: &dyn Any = s;`) and calls `a.downcast_ref::<bash::Scanner>()`.
- Asked by: the grammar stream (bead gositter-rs-vjz.12), through the coordinator.
