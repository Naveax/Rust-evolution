use crate::LowerError;
use evo_diagnostics::set_related_location;
use evo_lexer::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReferenceSource {
    Parameter(String),
    LocalOwner(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReferenceProvenance {
    source: ReferenceSource,
    origin_span: Span,
}

impl ReferenceProvenance {
    pub(crate) fn parameter(name: String, span: Span) -> Self {
        Self {
            source: ReferenceSource::Parameter(name),
            origin_span: span,
        }
    }

    pub(crate) fn local_owner(name: String, span: Span) -> Self {
        Self {
            source: ReferenceSource::LocalOwner(name),
            origin_span: span,
        }
    }

    pub(crate) fn source(&self) -> &ReferenceSource {
        &self.source
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerOperation {
    Move,
    Reinitialize,
}

impl OwnerOperation {
    const fn verb(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Reinitialize => "reinitialize",
        }
    }
}

#[derive(Debug, Clone)]
struct ReferenceBinding {
    provenance: ReferenceProvenance,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ReferenceTracker {
    bindings: HashMap<String, ReferenceBinding>,
}

impl ReferenceTracker {
    pub(crate) fn define_parameter(&mut self, name: String, span: Span) {
        let previous = self.bindings.insert(
            name.clone(),
            ReferenceBinding {
                provenance: ReferenceProvenance::parameter(name, span),
            },
        );
        debug_assert!(previous.is_none());
    }

    pub(crate) fn define_reference(
        &mut self,
        name: String,
        provenance: ReferenceProvenance,
    ) {
        let previous = self
            .bindings
            .insert(name, ReferenceBinding { provenance });
        debug_assert!(previous.is_none());
    }

    pub(crate) fn forget(&mut self, name: &str) {
        let _ = self.bindings.remove(name);
    }

    pub(crate) fn provenance(&self, name: &str) -> Option<&ReferenceProvenance> {
        self.bindings.get(name).map(|binding| &binding.provenance)
    }

    pub(crate) fn ensure_owner_operation_allowed(
        &self,
        owner: &str,
        operation: OwnerOperation,
        span: Span,
    ) -> Result<(), LowerError> {
        let conflict = self
            .bindings
            .iter()
            .filter_map(|(reference_name, binding)| match binding.provenance.source() {
                ReferenceSource::LocalOwner(candidate) if candidate == owner => {
                    Some((reference_name.as_str(), &binding.provenance))
                }
                ReferenceSource::Parameter(_) | ReferenceSource::LocalOwner(_) => None,
            })
            .min_by(|(left_name, left), (right_name, right)| {
                left.origin_span
                    .start
                    .cmp(&right.origin_span.start)
                    .then_with(|| left_name.cmp(right_name))
            });

        let Some((reference_name, provenance)) = conflict else {
            return Ok(());
        };

        let message = format!(
            "cannot {} record local {owner:?} while immutable reference {reference_name:?} is still live",
            operation.verb()
        );
        let note = format!("immutable reference {reference_name:?} was created here");
        set_related_location(&message, span, &note, provenance.origin_span);
        Err(LowerError { message, span })
    }

    pub(crate) fn validate_return_provenance(
        &self,
        provenance: &ReferenceProvenance,
        expected_parameter: &str,
        span: Span,
    ) -> Result<(), LowerError> {
        match provenance.source() {
            ReferenceSource::Parameter(name) if name == expected_parameter => Ok(()),
            ReferenceSource::Parameter(name) => {
                let message = format!(
                    "returned immutable reference is tied to parameter {name:?}, but v0 return provenance is tied to {expected_parameter:?}"
                );
                set_related_location(
                    &message,
                    span,
                    "reference provenance originates here",
                    provenance.origin_span,
                );
                Err(LowerError { message, span })
            }
            ReferenceSource::LocalOwner(owner) => {
                let message = format!(
                    "cannot return immutable reference derived from local owner {owner:?}"
                );
                set_related_location(
                    &message,
                    span,
                    "local-owner reference was created here",
                    provenance.origin_span,
                );
                Err(LowerError { message, span })
            }
        }
    }
}
