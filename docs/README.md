# Rust Evolution Documentation Map

Use this file to avoid mixing current implementation, project state, roadmap, and long-term vision.

## Start / continuation

- `../AGENTS.md` — mandatory agent, CI and continuation contract.
- `PROJECT_STATE.md` — verified current project status.
- `NEXT_ACTION.md` — exact active continuation point.
- `CONTINUATION_PROTOCOL.md` — new-session recovery sequence.
- `HANDOFF_CHECKLIST.md` — end-of-session maintenance checklist.

## Current implementation truth

- `LANGUAGE_SPEC_V0.md` — implemented Evolution semantics on `main`.
- `ARCHITECTURE.md` — current/near-term compiler architecture boundaries.
- `PERFORMANCE_CONTRACT.md` — non-negotiable native runtime acceptance rule.
- `BENCHMARKING.md` — differential correctness/performance methodology.

## Durable governance / design

- `DECISIONS.md` — architecture/language decisions future sessions should preserve unless superseded by evidence.
- `ROADMAP.md` — staged development roadmap.
- `LANGUAGE_DESIGN.md` — language design exploration.
- `VISION.md` — original high-level project vision.

## Omni long-term architecture

- `OMNI_VISION.md` — integrated full-stack north star based on the user-authored Omni update spec.
- `COST_MODEL.md` — ZERO / EXPLICIT / MANAGED cost direction.
- `PROFILE_MODEL.md` — Core vs Profile/Capability/Library/Runtime/Backend/Tooling model.

## Historical audit

- `AUDIT_2026-08-24.md` — early repository/planning audit.

## Research

- `../research/WEAKNESS_MAP.md` — Rust weakness taxonomy/backlog.
- `../research/languages/README.md` — systematic external-language/ecosystem idea research template.

## Rule of interpretation

When documents conflict:

`tests + main code` > `LANGUAGE_SPEC_V0` > current PR/CI evidence > `PROJECT_STATE/NEXT_ACTION` > `DECISIONS` > `ROADMAP` > `OMNI_VISION`.

The distinction is deliberate: **spec = proven behavior, state = current work, roadmap = planned sequence, vision = long-term target.**