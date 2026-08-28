use crate::record_environment::{
    ExecutableEnumIr, ExecutableEnumProgramIr, ExecutableEnumVariantIr, ExecutableExprIr,
    ExecutableExprKind, ExecutableFunctionIr, ExecutableMatchArmIr, ExecutableMatchBindingIr,
    ExecutableOwnershipMode, ExecutableParameterIr, ExecutableRecordFieldIr,
    ExecutableRecordFieldValueIr, ExecutableRecordIr, ExecutableStmtIr, ExecutableStmtKind,
    ExecutableValueType,
};
use crate::{BinaryOp, Program};
use evo_lexer::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumCodegenValueType<'a> {
    Integer,
    String,
    Bool,
    Record(&'a str),
    Enum(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumCodegenOwnershipMode {
    Inspect,
    Consume,
}

#[derive(Clone, Copy)]
pub struct EnumCodegenProgramView<'a> {
    inner: &'a ExecutableEnumProgramIr,
}

#[derive(Clone, Copy)]
pub struct EnumCodegenEnumListView<'a> {
    inner: &'a [ExecutableEnumIr],
}

#[derive(Clone, Copy)]
pub struct EnumCodegenEnumView<'a> {
    inner: &'a ExecutableEnumIr,
}

#[derive(Clone, Copy)]
pub struct EnumCodegenVariantListView<'a> {
    inner: &'a [ExecutableEnumVariantIr],
}

#[derive(Clone, Copy)]
pub struct EnumCodegenVariantView<'a> {
    inner: &'a ExecutableEnumVariantIr,
}

#[derive(Clone, Copy)]
pub struct EnumCodegenRecordListView<'a> {
    inner: &'a [ExecutableRecordIr],
}

#[derive(Clone, Copy)]
pub struct EnumCodegenRecordView<'a> {
    inner: &'a ExecutableRecordIr,
}

#[derive(Clone, Copy)]
pub struct EnumCodegenRecordFieldListView<'a> {
    inner: &'a [ExecutableRecordFieldIr],
}

#[derive(Clone, Copy)]
pub struct EnumCodegenRecordFieldView<'a> {
    inner: &'a ExecutableRecordFieldIr,
}

#[derive(Clone, Copy)]
pub struct EnumCodegenFunctionListView<'a> {
    inner: &'a [ExecutableFunctionIr],
}

#[derive(Clone, Copy)]
pub struct EnumCodegenFunctionView<'a> {
    inner: &'a ExecutableFunctionIr,
}

#[derive(Clone, Copy)]
pub struct EnumCodegenParameterListView<'a> {
    inner: &'a [ExecutableParameterIr],
}

#[derive(Clone, Copy)]
pub struct EnumCodegenParameterView<'a> {
    inner: &'a ExecutableParameterIr,
}

#[derive(Clone, Copy)]
pub struct EnumCodegenStmtListView<'a> {
    inner: &'a [ExecutableStmtIr],
}

#[derive(Clone, Copy)]
pub struct EnumCodegenStmtView<'a> {
    inner: &'a ExecutableStmtIr,
}

#[derive(Clone, Copy)]
pub struct EnumCodegenMatchArmListView<'a> {
    inner: &'a [ExecutableMatchArmIr],
}

#[derive(Clone, Copy)]
pub struct EnumCodegenMatchArmView<'a> {
    inner: &'a ExecutableMatchArmIr,
}

#[derive(Clone, Copy)]
pub struct EnumCodegenMatchBindingView<'a> {
    inner: &'a ExecutableMatchBindingIr,
}

#[derive(Clone, Copy)]
pub struct EnumCodegenExprListView<'a> {
    inner: &'a [ExecutableExprIr],
}

#[derive(Clone, Copy)]
pub struct EnumCodegenExprView<'a> {
    inner: &'a ExecutableExprIr,
}

#[derive(Clone, Copy)]
pub struct EnumCodegenRecordFieldValueListView<'a> {
    inner: &'a [ExecutableRecordFieldValueIr],
}

#[derive(Clone, Copy)]
pub struct EnumCodegenRecordFieldValueView<'a> {
    inner: &'a ExecutableRecordFieldValueIr,
}

#[derive(Clone, Copy)]
pub enum EnumCodegenStmtKindView<'a> {
    Let {
        name: &'a str,
        mutable: bool,
        expr: EnumCodegenExprView<'a>,
    },
    Assign {
        name: &'a str,
        expr: EnumCodegenExprView<'a>,
    },
    Print(EnumCodegenExprView<'a>),
    Return(EnumCodegenExprView<'a>),
    Repeat {
        count: EnumCodegenExprView<'a>,
        body: EnumCodegenStmtListView<'a>,
    },
    If {
        condition: EnumCodegenExprView<'a>,
        then_body: EnumCodegenStmtListView<'a>,
        else_body: EnumCodegenStmtListView<'a>,
    },
    Match {
        value: EnumCodegenExprView<'a>,
        enum_name: &'a str,
        arms: EnumCodegenMatchArmListView<'a>,
        all_arms_return: bool,
    },
}

#[derive(Clone, Copy)]
pub enum EnumCodegenExprKindView<'a> {
    Integer(i64),
    String(&'a str),
    Bool(bool),
    Local {
        name: &'a str,
        value_type: EnumCodegenValueType<'a>,
        ownership: EnumCodegenOwnershipMode,
    },
    Call {
        name: &'a str,
        arguments: EnumCodegenExprListView<'a>,
    },
    RecordConstruct {
        name: &'a str,
        fields: EnumCodegenRecordFieldValueListView<'a>,
    },
    EnumConstruct {
        enum_name: &'a str,
        variant_name: &'a str,
        payload_type: Option<EnumCodegenValueType<'a>>,
        payload: Option<EnumCodegenExprView<'a>>,
    },
    FieldAccess {
        base: EnumCodegenExprView<'a>,
        field: &'a str,
    },
    InputInt,
    LogicalNot(EnumCodegenExprView<'a>),
    UnaryMinus(EnumCodegenExprView<'a>),
    Binary {
        left: EnumCodegenExprView<'a>,
        op: BinaryOp,
        right: EnumCodegenExprView<'a>,
    },
}

impl Program {
    #[must_use]
    pub fn enum_codegen_view(&self) -> Option<EnumCodegenProgramView<'_>> {
        self.enum_program
            .as_ref()
            .map(|inner| EnumCodegenProgramView { inner })
    }
}

impl<'a> EnumCodegenProgramView<'a> {
    #[must_use]
    pub fn enums(self) -> EnumCodegenEnumListView<'a> {
        EnumCodegenEnumListView {
            inner: &self.inner.enums,
        }
    }

    #[must_use]
    pub fn records(self) -> EnumCodegenRecordListView<'a> {
        EnumCodegenRecordListView {
            inner: &self.inner.records,
        }
    }

    #[must_use]
    pub fn functions(self) -> EnumCodegenFunctionListView<'a> {
        EnumCodegenFunctionListView {
            inner: &self.inner.functions,
        }
    }

    #[must_use]
    pub fn statements(self) -> EnumCodegenStmtListView<'a> {
        EnumCodegenStmtListView {
            inner: &self.inner.statements,
        }
    }
}

macro_rules! impl_list_view {
    ($list:ident, $item:ident) => {
        impl<'a> $list<'a> {
            #[must_use]
            pub fn len(self) -> usize {
                self.inner.len()
            }

            #[must_use]
            pub fn is_empty(self) -> bool {
                self.inner.is_empty()
            }

            #[must_use]
            pub fn get(self, index: usize) -> Option<$item<'a>> {
                self.inner.get(index).map(|inner| $item { inner })
            }

            #[must_use]
            pub fn iter(
                self,
            ) -> impl ExactSizeIterator<Item = $item<'a>> + DoubleEndedIterator + 'a {
                self.inner.iter().map(|inner| $item { inner })
            }
        }
    };
}

impl_list_view!(EnumCodegenEnumListView, EnumCodegenEnumView);
impl_list_view!(EnumCodegenVariantListView, EnumCodegenVariantView);
impl_list_view!(EnumCodegenRecordListView, EnumCodegenRecordView);
impl_list_view!(EnumCodegenRecordFieldListView, EnumCodegenRecordFieldView);
impl_list_view!(EnumCodegenFunctionListView, EnumCodegenFunctionView);
impl_list_view!(EnumCodegenParameterListView, EnumCodegenParameterView);
impl_list_view!(EnumCodegenStmtListView, EnumCodegenStmtView);
impl_list_view!(EnumCodegenMatchArmListView, EnumCodegenMatchArmView);
impl_list_view!(EnumCodegenExprListView, EnumCodegenExprView);
impl_list_view!(
    EnumCodegenRecordFieldValueListView,
    EnumCodegenRecordFieldValueView
);

impl<'a> EnumCodegenEnumView<'a> {
    #[must_use]
    pub fn name(self) -> &'a str {
        &self.inner.name
    }

    #[must_use]
    pub fn variants(self) -> EnumCodegenVariantListView<'a> {
        EnumCodegenVariantListView {
            inner: &self.inner.variants,
        }
    }

    #[must_use]
    pub fn span(self) -> Span {
        self.inner.span
    }
}

impl<'a> EnumCodegenVariantView<'a> {
    #[must_use]
    pub fn name(self) -> &'a str {
        &self.inner.name
    }

    #[must_use]
    pub fn payload_type(self) -> Option<EnumCodegenValueType<'a>> {
        self.inner.payload_type.as_ref().map(value_type_view)
    }

    #[must_use]
    pub fn span(self) -> Span {
        self.inner.span
    }
}

impl<'a> EnumCodegenRecordView<'a> {
    #[must_use]
    pub fn name(self) -> &'a str {
        &self.inner.name
    }

    #[must_use]
    pub fn fields(self) -> EnumCodegenRecordFieldListView<'a> {
        EnumCodegenRecordFieldListView {
            inner: &self.inner.fields,
        }
    }

    #[must_use]
    pub fn span(self) -> Span {
        self.inner.span
    }
}

impl<'a> EnumCodegenRecordFieldView<'a> {
    #[must_use]
    pub fn name(self) -> &'a str {
        &self.inner.name
    }

    #[must_use]
    pub fn value_type(self) -> EnumCodegenValueType<'a> {
        value_type_view(&self.inner.value_type)
    }

    #[must_use]
    pub fn span(self) -> Span {
        self.inner.span
    }
}

impl<'a> EnumCodegenFunctionView<'a> {
    #[must_use]
    pub fn name(self) -> &'a str {
        &self.inner.name
    }

    #[must_use]
    pub fn parameters(self) -> EnumCodegenParameterListView<'a> {
        EnumCodegenParameterListView {
            inner: &self.inner.parameters,
        }
    }

    #[must_use]
    pub fn return_type(self) -> EnumCodegenValueType<'a> {
        value_type_view(&self.inner.return_type)
    }

    #[must_use]
    pub fn body(self) -> EnumCodegenStmtListView<'a> {
        EnumCodegenStmtListView {
            inner: &self.inner.body,
        }
    }

    #[must_use]
    pub fn span(self) -> Span {
        self.inner.span
    }
}

impl<'a> EnumCodegenParameterView<'a> {
    #[must_use]
    pub fn name(self) -> &'a str {
        &self.inner.name
    }

    #[must_use]
    pub fn value_type(self) -> EnumCodegenValueType<'a> {
        value_type_view(&self.inner.value_type)
    }

    #[must_use]
    pub fn mutable(self) -> bool {
        self.inner.mutable
    }

    #[must_use]
    pub fn span(self) -> Span {
        self.inner.span
    }
}

impl<'a> EnumCodegenStmtView<'a> {
    #[must_use]
    pub fn kind(self) -> EnumCodegenStmtKindView<'a> {
        match &self.inner.kind {
            ExecutableStmtKind::Let {
                name,
                mutable,
                expr,
            } => EnumCodegenStmtKindView::Let {
                name,
                mutable: *mutable,
                expr: EnumCodegenExprView { inner: expr },
            },
            ExecutableStmtKind::Assign { name, expr } => EnumCodegenStmtKindView::Assign {
                name,
                expr: EnumCodegenExprView { inner: expr },
            },
            ExecutableStmtKind::Print(expr) => {
                EnumCodegenStmtKindView::Print(EnumCodegenExprView { inner: expr })
            }
            ExecutableStmtKind::Return(expr) => {
                EnumCodegenStmtKindView::Return(EnumCodegenExprView { inner: expr })
            }
            ExecutableStmtKind::Repeat { count, body } => EnumCodegenStmtKindView::Repeat {
                count: EnumCodegenExprView { inner: count },
                body: EnumCodegenStmtListView { inner: body },
            },
            ExecutableStmtKind::If {
                condition,
                then_body,
                else_body,
            } => EnumCodegenStmtKindView::If {
                condition: EnumCodegenExprView { inner: condition },
                then_body: EnumCodegenStmtListView { inner: then_body },
                else_body: EnumCodegenStmtListView { inner: else_body },
            },
            ExecutableStmtKind::Match {
                value,
                enum_name,
                arms,
                all_arms_return,
            } => EnumCodegenStmtKindView::Match {
                value: EnumCodegenExprView { inner: value },
                enum_name,
                arms: EnumCodegenMatchArmListView { inner: arms },
                all_arms_return: *all_arms_return,
            },
        }
    }

    #[must_use]
    pub fn span(self) -> Span {
        self.inner.span
    }
}

impl<'a> EnumCodegenMatchArmView<'a> {
    #[must_use]
    pub fn enum_name(self) -> &'a str {
        &self.inner.enum_name
    }

    #[must_use]
    pub fn variant_name(self) -> &'a str {
        &self.inner.variant_name
    }

    #[must_use]
    pub fn binding(self) -> Option<EnumCodegenMatchBindingView<'a>> {
        self.inner
            .binding
            .as_ref()
            .map(|inner| EnumCodegenMatchBindingView { inner })
    }

    #[must_use]
    pub fn body(self) -> EnumCodegenStmtListView<'a> {
        EnumCodegenStmtListView {
            inner: &self.inner.body,
        }
    }

    #[must_use]
    pub fn span(self) -> Span {
        self.inner.span
    }
}

impl<'a> EnumCodegenMatchBindingView<'a> {
    #[must_use]
    pub fn name(self) -> &'a str {
        &self.inner.name
    }

    #[must_use]
    pub fn value_type(self) -> EnumCodegenValueType<'a> {
        value_type_view(&self.inner.value_type)
    }

    #[must_use]
    pub fn mutable(self) -> bool {
        self.inner.mutable
    }

    #[must_use]
    pub fn span(self) -> Span {
        self.inner.span
    }
}

impl<'a> EnumCodegenExprView<'a> {
    #[must_use]
    pub fn kind(self) -> EnumCodegenExprKindView<'a> {
        match &self.inner.kind {
            ExecutableExprKind::Integer(value) => EnumCodegenExprKindView::Integer(*value),
            ExecutableExprKind::String(value) => EnumCodegenExprKindView::String(value),
            ExecutableExprKind::Bool(value) => EnumCodegenExprKindView::Bool(*value),
            ExecutableExprKind::Local {
                name,
                value_type,
                ownership,
            } => EnumCodegenExprKindView::Local {
                name,
                value_type: value_type_view(value_type),
                ownership: ownership_mode_view(*ownership),
            },
            ExecutableExprKind::Call { name, arguments } => EnumCodegenExprKindView::Call {
                name,
                arguments: EnumCodegenExprListView { inner: arguments },
            },
            ExecutableExprKind::RecordConstruct { name, fields } => {
                EnumCodegenExprKindView::RecordConstruct {
                    name,
                    fields: EnumCodegenRecordFieldValueListView { inner: fields },
                }
            }
            ExecutableExprKind::EnumConstruct {
                enum_name,
                variant_name,
                payload_type,
                payload,
            } => EnumCodegenExprKindView::EnumConstruct {
                enum_name,
                variant_name,
                payload_type: payload_type.as_ref().map(value_type_view),
                payload: payload
                    .as_deref()
                    .map(|inner| EnumCodegenExprView { inner }),
            },
            ExecutableExprKind::FieldAccess { base, field } => {
                EnumCodegenExprKindView::FieldAccess {
                    base: EnumCodegenExprView { inner: base },
                    field,
                }
            }
            ExecutableExprKind::InputInt => EnumCodegenExprKindView::InputInt,
            ExecutableExprKind::LogicalNot(inner) => {
                EnumCodegenExprKindView::LogicalNot(EnumCodegenExprView { inner })
            }
            ExecutableExprKind::UnaryMinus(inner) => {
                EnumCodegenExprKindView::UnaryMinus(EnumCodegenExprView { inner })
            }
            ExecutableExprKind::Binary { left, op, right } => EnumCodegenExprKindView::Binary {
                left: EnumCodegenExprView { inner: left },
                op: *op,
                right: EnumCodegenExprView { inner: right },
            },
        }
    }

    #[must_use]
    pub fn span(self) -> Span {
        self.inner.span
    }
}

impl<'a> EnumCodegenRecordFieldValueView<'a> {
    #[must_use]
    pub fn name(self) -> &'a str {
        &self.inner.name
    }

    #[must_use]
    pub fn value(self) -> EnumCodegenExprView<'a> {
        EnumCodegenExprView {
            inner: &self.inner.value,
        }
    }

    #[must_use]
    pub fn span(self) -> Span {
        self.inner.span
    }
}

fn value_type_view(value_type: &ExecutableValueType) -> EnumCodegenValueType<'_> {
    match value_type {
        ExecutableValueType::Integer => EnumCodegenValueType::Integer,
        ExecutableValueType::String => EnumCodegenValueType::String,
        ExecutableValueType::Bool => EnumCodegenValueType::Bool,
        ExecutableValueType::Record(name) => EnumCodegenValueType::Record(name),
        ExecutableValueType::Enum(name) => EnumCodegenValueType::Enum(name),
    }
}

const fn ownership_mode_view(mode: ExecutableOwnershipMode) -> EnumCodegenOwnershipMode {
    match mode {
        ExecutableOwnershipMode::Inspect => EnumCodegenOwnershipMode::Inspect,
        ExecutableOwnershipMode::Consume => EnumCodegenOwnershipMode::Consume,
    }
}
