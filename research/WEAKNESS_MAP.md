# Rust Weakness Map

This document is the repository-level taxonomy for the project's starting weakness inventory. GitHub issue #6 contains the exhaustive checkbox backlog.

## P0 / Core areas

### Developer experience
- rapid prototyping
- scripting and one-off programs
- REPL/interactive development
- edit-compile-run cycle
- learning curve
- diagnostics
- refactoring ergonomics

### Language ergonomics
- ownership/borrowing learning and expression
- lifetime management
- complex generics/traits
- shared mutable state patterns
- self-referential/cyclic/intrusive structures

### Build/compile
- cold/warm/incremental compilation
- dependency builds
- release build time
- generic/monomorphization cost
- proc-macro compile/debug cost
- compile memory
- workspace scaling
- binary size

## P1 / Runtime and tooling

### Async/concurrency
- async ergonomics
- Pin/Unpin
- Send/Sync diagnostics
- task lifecycle/cancellation
- shared-state patterns

### IDE/debug/tooling
- advanced IDE workflows
- debugger experience
- proc-macro debugging
- profiling/coverage/sanitizers

### FFI/platform
- native library integration
- C++ interoperability/templates
- legacy C++ integration
- FFI safety
- cross-compilation
- old/niche targets
- vendor/proprietary SDKs

## P2 / Application ecosystems

### GUI/Desktop
- desktop GUI
- GUI designers
- WYSIWYG tooling

### Mobile
- Android UI/integration
- iOS UI/integration
- Swift/Kotlin ecosystem gap

### Web/application development
- frontend/DOM workflows
- simple backend/CRUD speed
- serverless
- CMS/WordPress-like systems
- low/no-code integrations
- ORM-heavy development

### Automation/enterprise
- Office/Excel automation
- Python-scale automation ecosystem
- Java/.NET-scale enterprise framework ecosystem
- commercial/vendor SDK coverage

## P3 / Specialized domains

### Data science / ML / scientific
- data analysis/statistics
- notebooks/Jupyter-style work
- academic computing
- ML research
- CUDA/GPU/GPGPU
- scientific package ecosystem

### Game development
- AAA workflows
- Unity/Unreal integration
- rapid game prototyping
- visual editors
- C++-scale game ecosystem

### Embedded
- vendor SDK-dependent targets
- microcontroller ecosystem gaps
- proprietary drivers
- niche hardware

## Dynamic/runtime model gaps

Research areas that may conflict with Rust's design and therefore require extra caution:

- dynamic metaprogramming
- runtime reflection
- highly dynamic data models
- dynamic plugin systems
- inheritance-heavy OOP
- GC-friendly cyclic object graphs

These are not automatically goals to copy from other languages. Each must be analyzed for whether it is compatible with the project's safety/performance invariants.

## Adoption and migration

- Rust talent availability
- junior onboarding cost
- large-team migration cost
- C/C++ migration cost
- ecosystem/job-market scale

## Processing template

Every weakness is eventually assigned:

- Problem
- Category
- Layer
- Severity
- Frequency
- Affected users
- Root cause
- Fixability
- Complexity
- Security impact
- Runtime impact
- Proposed solution
- Benchmark/test
- Decision

Initial priority heuristic:

`Priority = Impact × Frequency × User Base × Fixability ÷ Complexity`

This heuristic never overrides the safety and performance contracts.
