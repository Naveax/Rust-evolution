# Research Workflow Retirement

This file records why two pre-production research workflows are no longer permanent CI gates after their findings were implemented.

## Borrow inference research

The historical `Borrow inference research` workflow asserted that selected fixtures still demonstrated the pre-production move friction that motivated inferred call-duration `SharedBorrow`. Production shared-borrow inference has since been implemented, so that assertion is intentionally false for accepted cases. The historical test fixtures and reports remain in the repository as research evidence, but the workflow is retired rather than rewritten to pretend the old precondition still holds.

## Immutable reference surface research

The historical `Immutable reference surface research` workflow validated that `&` was not yet production syntax and selected `SURFACE-CANDIDATE / PUNCTUATION-AMPERSAND`. Production issue #104 implements that accepted surface. Keeping the pre-adoption workflow as a `main` gate would make successful adoption look like a research regression, so the workflow is retired while `docs/IMMUTABLE_REFERENCE_SURFACE_RESEARCH.md`, the ignored research test, accepted artifact metadata, and commit provenance remain historical evidence.

Production behavior is validated by normal cross-platform CI and permanent feature tests instead of by these research-only gates.
