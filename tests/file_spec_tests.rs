use std::cell::RefCell;
use std::rc::Rc;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::code_node::{BlockIntent, CodeNode};
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::import::{
    ImportAliasAssignment, ImportAliasConflictResolver, ImportAliasConflicts, ImportAliasRejection,
    ImportEntry, ImportGroup,
};
use sigil_stitch::lang::{CodeLang, RendererLang};
use sigil_stitch::spec::emittable::Emittable;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpecOrigin {
    Trace,
    Failure,
    SingleBlock,
    MultipleBlocks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportRewrite {
    Preserve,
    Remove,
    Replace,
    Introduce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourcePosition {
    Root,
    Nested,
    Sequence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceLocation {
    Unlabelled,
    ImportRewrite {
        rewrite: ImportRewrite,
        position: SourcePosition,
    },
    Header,
    StoredCode,
    MaterializedSingle,
    MaterializedMultiple(usize),
}

const SOURCE_MARKERS: &[(SourceLocation, &str)] = &[
    (
        SourceLocation::ImportRewrite {
            rewrite: ImportRewrite::Preserve,
            position: SourcePosition::Root,
        },
        "__source_root_preserve__",
    ),
    (
        SourceLocation::ImportRewrite {
            rewrite: ImportRewrite::Remove,
            position: SourcePosition::Root,
        },
        "__source_root_remove__",
    ),
    (
        SourceLocation::ImportRewrite {
            rewrite: ImportRewrite::Replace,
            position: SourcePosition::Root,
        },
        "__source_root_replace__",
    ),
    (
        SourceLocation::ImportRewrite {
            rewrite: ImportRewrite::Introduce,
            position: SourcePosition::Root,
        },
        "__source_root_introduce__",
    ),
    (
        SourceLocation::ImportRewrite {
            rewrite: ImportRewrite::Preserve,
            position: SourcePosition::Nested,
        },
        "__source_nested_preserve__",
    ),
    (
        SourceLocation::ImportRewrite {
            rewrite: ImportRewrite::Remove,
            position: SourcePosition::Nested,
        },
        "__source_nested_remove__",
    ),
    (
        SourceLocation::ImportRewrite {
            rewrite: ImportRewrite::Replace,
            position: SourcePosition::Nested,
        },
        "__source_nested_replace__",
    ),
    (
        SourceLocation::ImportRewrite {
            rewrite: ImportRewrite::Introduce,
            position: SourcePosition::Nested,
        },
        "__source_nested_introduce__",
    ),
    (
        SourceLocation::ImportRewrite {
            rewrite: ImportRewrite::Preserve,
            position: SourcePosition::Sequence,
        },
        "__source_sequence_preserve__",
    ),
    (
        SourceLocation::ImportRewrite {
            rewrite: ImportRewrite::Remove,
            position: SourcePosition::Sequence,
        },
        "__source_sequence_remove__",
    ),
    (
        SourceLocation::ImportRewrite {
            rewrite: ImportRewrite::Replace,
            position: SourcePosition::Sequence,
        },
        "__source_sequence_replace__",
    ),
    (
        SourceLocation::ImportRewrite {
            rewrite: ImportRewrite::Introduce,
            position: SourcePosition::Sequence,
        },
        "__source_sequence_introduce__",
    ),
    (SourceLocation::Header, "__source_header__"),
    (SourceLocation::StoredCode, "__source_stored_code__"),
    (
        SourceLocation::MaterializedSingle,
        "__source_materialized_single__",
    ),
    (
        SourceLocation::MaterializedMultiple(0),
        "__source_materialized_multiple_0__",
    ),
    (
        SourceLocation::MaterializedMultiple(1),
        "__source_materialized_multiple_1__",
    ),
];

impl SourceLocation {
    fn marker(self) -> &'static str {
        if self == Self::Unlabelled {
            return "";
        }

        SOURCE_MARKERS
            .iter()
            .find_map(|(location, marker)| (*location == self).then_some(*marker))
            .unwrap_or_else(|| panic!("no source marker registered for {self:?}"))
    }

    fn from_marker(marker: &str) -> Option<Self> {
        SOURCE_MARKERS
            .iter()
            .find_map(|(location, candidate)| (*candidate == marker).then_some(*location))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PipelineEvent {
    Validate(SpecOrigin),
    Emit(SpecOrigin),
    Rewrite(SourceLocation),
    Lower(TypeName),
    RenderStatement,
    RenderOpen,
    RenderTransition,
    RenderClose,
}

#[derive(Debug, Clone)]
struct PipelineLang {
    events: Rc<RefCell<Vec<PipelineEvent>>>,
}

impl PipelineLang {
    fn new(events: Rc<RefCell<Vec<PipelineEvent>>>) -> Self {
        Self { events }
    }
}

fn take_source_location(nodes: &mut Vec<CodeNode>) -> Option<SourceLocation> {
    let mut index = 0;
    while index < nodes.len() {
        let location = match &mut nodes[index] {
            CodeNode::Literal(text) => SourceLocation::from_marker(text),
            CodeNode::Nested(block) => take_source_location(block.nodes_mut()),
            CodeNode::Sequence(children) => take_source_location(children),
            _ => None,
        };
        if let Some(location) = location {
            if matches!(&nodes[index], CodeNode::Literal(text) if SourceLocation::from_marker(text).is_some())
            {
                nodes.remove(index);
            }
            return Some(location);
        }
        index += 1;
    }
    None
}

fn rewrite_pipeline_nodes(nodes: &mut Vec<CodeNode>) {
    let mut index = 0;
    while index < nodes.len() {
        if matches!(
            &nodes[index],
            CodeNode::TypeRef(TypeName::Importable { name, .. }) if name == "Removed"
        ) {
            nodes.remove(index);
            continue;
        }

        if matches!(
            &nodes[index],
            CodeNode::TypeRef(TypeName::Importable { name, .. }) if name == "BeforeReplace"
        ) {
            nodes[index] =
                CodeNode::TypeRef(TypeName::importable_type("./replacement", "Replaced"));
        } else if matches!(&nodes[index], CodeNode::Literal(text) if text == "INSERT_IMPORT") {
            nodes[index] = CodeNode::TypeRef(TypeName::importable_type("./rewritten", "User"));
        } else if matches!(&nodes[index], CodeNode::Literal(text) if text == "INVALID_REWRITE") {
            nodes[index] = CodeNode::Dedent;
        } else {
            match &mut nodes[index] {
                CodeNode::Nested(block) => rewrite_pipeline_nodes(block.nodes_mut()),
                CodeNode::Sequence(children) => rewrite_pipeline_nodes(children),
                _ => {}
            }
        }
        index += 1;
    }
}

impl RendererLang for PipelineLang {
    fn file_extension(&self) -> &str {
        "pipeline"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn rewrite_nodes(&self, nodes: &mut Vec<CodeNode>) {
        let location = take_source_location(nodes).unwrap_or(SourceLocation::Unlabelled);
        self.events
            .borrow_mut()
            .push(PipelineEvent::Rewrite(location));
        rewrite_pipeline_nodes(nodes);
    }

    fn lower_type_name(&self, type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
        self.events
            .borrow_mut()
            .push(PipelineEvent::Lower(type_name.clone()));
        if matches!(type_name, TypeName::Primitive(name) if name == "unsupported") {
            return Err(SigilStitchError::UnsupportedTypeName {
                language: self.file_extension().to_string(),
                context: String::new(),
                reason: "intentional target rejection".to_string(),
            });
        }
        if matches!(type_name, TypeName::Primitive(name) if name == "invalid-lowering") {
            return CodeBlock::of("%]", ());
        }
        let terminal = match type_name {
            TypeName::Primitive(name) if name == "MetadataDerived" => {
                TypeName::importable_type("./metadata-derived", "MetadataDerived")
            }
            TypeName::Generic { base, .. } if matches!(base.as_ref(), TypeName::Primitive(name) if name == "MetadataCompound") => {
                TypeName::importable_type("./metadata-compound", "MetadataCompound")
            }
            TypeName::Importable {
                module,
                name,
                is_type_only,
                alias,
                ..
            } => TypeName::Importable {
                module: module.clone(),
                name: name.clone(),
                is_type_only: *is_type_only,
                alias: alias.clone(),
                qualified: false,
            },
            TypeName::Primitive(name) | TypeName::Raw(name) => TypeName::primitive(name),
            _ => TypeName::primitive("LoweredType"),
        };
        CodeBlock::of("%T", (terminal,))
    }

    fn render_statement_end(&self) -> Result<&str, SigilStitchError> {
        self.events
            .borrow_mut()
            .push(PipelineEvent::RenderStatement);
        Ok(";")
    }

    fn render_block_open(
        &self,
        _intent: BlockIntent,
        _condition: &str,
    ) -> Result<&str, SigilStitchError> {
        self.events.borrow_mut().push(PipelineEvent::RenderOpen);
        Ok("{")
    }

    fn render_block_close(
        &self,
        _intent: BlockIntent,
        _condition: &str,
    ) -> Result<&str, SigilStitchError> {
        self.events.borrow_mut().push(PipelineEvent::RenderClose);
        Ok("}")
    }

    fn render_branch_transition(
        &self,
        _intent: BlockIntent,
        _condition: &str,
    ) -> Result<String, SigilStitchError> {
        self.events
            .borrow_mut()
            .push(PipelineEvent::RenderTransition);
        Ok("} ".to_string())
    }
}

impl CodeLang for PipelineLang {
    fn render_imports(&self, imports: &ImportGroup) -> String {
        imports
            .entries()
            .iter()
            .filter(|entry| !entry.is_side_effect && !entry.is_wildcard)
            .map(|entry| {
                format!(
                    "import {} as {} from {}",
                    entry.name,
                    entry.resolved_name(),
                    entry.module
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, Copy)]
enum PipelineSpecFailure {
    None,
    Validation,
    Emission,
}

#[derive(Debug)]
struct PipelineSpec {
    events: Rc<RefCell<Vec<PipelineEvent>>>,
    origin: SpecOrigin,
    blocks: Vec<CodeBlock>,
    failure: PipelineSpecFailure,
}

impl PipelineSpec {
    fn one_block(
        events: Rc<RefCell<Vec<PipelineEvent>>>,
        origin: SpecOrigin,
        block: CodeBlock,
        failure: PipelineSpecFailure,
    ) -> Self {
        Self {
            events,
            origin,
            blocks: vec![block],
            failure,
        }
    }

    fn multiple_blocks(
        events: Rc<RefCell<Vec<PipelineEvent>>>,
        origin: SpecOrigin,
        blocks: Vec<CodeBlock>,
    ) -> Self {
        Self {
            events,
            origin,
            blocks,
            failure: PipelineSpecFailure::None,
        }
    }
}

impl Emittable for PipelineSpec {
    fn collect_validation_errors(&self, _lang: &dyn CodeLang, errors: &mut Vec<SigilStitchError>) {
        self.events
            .borrow_mut()
            .push(PipelineEvent::Validate(self.origin));
        if matches!(self.failure, PipelineSpecFailure::Validation) {
            errors.push(SigilStitchError::Render {
                context: "PipelineSpec::validate".to_string(),
                message: "intentional validation failure".to_string(),
            });
        }
    }

    fn emit_members(&self, _lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
        self.events
            .borrow_mut()
            .push(PipelineEvent::Emit(self.origin));
        if matches!(self.failure, PipelineSpecFailure::Emission) {
            return Err(SigilStitchError::Render {
                context: "PipelineSpec::emit_members".to_string(),
                message: "intentional emission failure".to_string(),
            });
        }
        Ok(self.blocks.clone())
    }
}

fn control_flow_with_type(type_name: TypeName) -> CodeBlock {
    let mut block = CodeBlock::builder();
    block.begin_control_flow("if ready", ());
    block.add_statement("value: %T", (type_name,));
    block.next_control_flow("else", ());
    block.add_statement("fallback", ());
    block.end_control_flow();
    block.build().unwrap()
}

fn located_source_node(location: SourceLocation, node: CodeNode) -> CodeBlock {
    let mut block = CodeBlock::of(location.marker(), ()).unwrap();
    block.nodes_mut().push(node);
    block
}

fn located_type(location: SourceLocation, type_name: TypeName) -> CodeBlock {
    located_source_node(location, CodeNode::TypeRef(type_name))
}

fn rewrite_operand(rewrite: ImportRewrite) -> CodeNode {
    match rewrite {
        ImportRewrite::Preserve => {
            CodeNode::TypeRef(TypeName::importable_type("./preserved", "Preserved"))
        }
        ImportRewrite::Remove => {
            CodeNode::TypeRef(TypeName::importable_type("./removed", "Removed"))
        }
        ImportRewrite::Replace => CodeNode::TypeRef(TypeName::importable_type(
            "./before-replace",
            "BeforeReplace",
        )),
        ImportRewrite::Introduce => CodeNode::Literal("INSERT_IMPORT".to_string()),
    }
}

fn expected_rewritten_type(rewrite: ImportRewrite) -> Option<TypeName> {
    match rewrite {
        ImportRewrite::Preserve => Some(TypeName::importable_type("./preserved", "Preserved")),
        ImportRewrite::Remove => None,
        ImportRewrite::Replace => Some(TypeName::importable_type("./replacement", "Replaced")),
        ImportRewrite::Introduce => Some(TypeName::importable_type("./rewritten", "User")),
    }
}

fn positioned_rewrite_block(rewrite: ImportRewrite, position: SourcePosition) -> CodeBlock {
    let location = SourceLocation::ImportRewrite { rewrite, position };
    let mut located = located_source_node(location, rewrite_operand(rewrite));

    match position {
        SourcePosition::Root => located,
        SourcePosition::Nested => CodeBlock::of("%L", (located,)).unwrap(),
        SourcePosition::Sequence => {
            let children = std::mem::take(located.nodes_mut());
            let mut root = CodeBlock::of("", ()).unwrap();
            root.nodes_mut().push(CodeNode::Sequence(children));
            root
        }
    }
}

struct ModuleResolver;

impl ImportAliasConflictResolver for ModuleResolver {
    fn resolve(
        &self,
        conflicts: &ImportAliasConflicts<'_>,
    ) -> Result<Vec<ImportAliasAssignment>, ImportAliasRejection> {
        Ok(conflicts
            .conflicts()
            .iter()
            .flat_map(|conflict| conflict.claims())
            .map(|claim| {
                let local_name = if claim.module() == "./models" {
                    "PrimaryUser"
                } else {
                    "SecondaryUser"
                };
                ImportAliasAssignment::new(claim.id(), local_name)
            })
            .collect())
    }
}

struct RejectingResolver;

impl ImportAliasConflictResolver for RejectingResolver {
    fn resolve(
        &self,
        _conflicts: &ImportAliasConflicts<'_>,
    ) -> Result<Vec<ImportAliasAssignment>, ImportAliasRejection> {
        Err(ImportAliasRejection::new("aliases disabled by caller"))
    }
}

#[test]
fn test_empty_file() {
    let file = FileSpec::builder("empty.ts").build().unwrap();
    let output = file.render(80).unwrap();
    assert!(output.is_empty() || output.trim().is_empty());
}

#[test]
fn test_simple_file_with_import() {
    let user = TypeName::importable_type("./models", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("const u: %T = getUser()", (user,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("user.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.contains("import type { User } from './models'"));
    assert!(output.contains("const u: User = getUser();"));
}

#[test]
fn file_validation_and_render_record_the_complete_pipeline_trace() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let type_name = TypeName::importable_type("./models", "User");
    let file = FileSpec::builder_with("trace.pipeline", PipelineLang::new(events.clone()))
        .add_spec(PipelineSpec::one_block(
            events.clone(),
            SpecOrigin::Trace,
            control_flow_with_type(type_name.clone()),
            PipelineSpecFailure::None,
        ))
        .build()
        .unwrap();

    file.validate().unwrap();
    assert_eq!(
        events.borrow().as_slice(),
        &[PipelineEvent::Validate(SpecOrigin::Trace)]
    );
    events.borrow_mut().clear();

    let output = file.render(80).unwrap();

    assert!(output.contains("import User as User from ./models"));
    assert!(output.contains("value: User"));
    assert_eq!(
        events.borrow().as_slice(),
        &[
            PipelineEvent::Validate(SpecOrigin::Trace),
            PipelineEvent::Emit(SpecOrigin::Trace),
            PipelineEvent::Rewrite(SourceLocation::Unlabelled),
            PipelineEvent::Lower(type_name),
            PipelineEvent::RenderOpen,
            PipelineEvent::RenderStatement,
            PipelineEvent::RenderTransition,
            PipelineEvent::RenderOpen,
            PipelineEvent::RenderStatement,
            PipelineEvent::RenderClose,
        ]
    );
}

#[test]
fn standalone_render_records_rewrite_lowering_and_terminal_renderer_events_once() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let type_name = TypeName::primitive("Value");
    let block = control_flow_with_type(type_name.clone());

    let output = block
        .render_standalone(&PipelineLang::new(events.clone()), 80)
        .unwrap();

    assert!(output.contains("value: Value"));
    assert_eq!(
        events.borrow().as_slice(),
        &[
            PipelineEvent::Rewrite(SourceLocation::Unlabelled),
            PipelineEvent::Lower(type_name),
            PipelineEvent::RenderOpen,
            PipelineEvent::RenderStatement,
            PipelineEvent::RenderTransition,
            PipelineEvent::RenderOpen,
            PipelineEvent::RenderStatement,
            PipelineEvent::RenderClose,
        ]
    );
}

#[test]
fn pipeline_failures_stop_before_every_later_event() {
    let cases = [
        (
            PipelineSpecFailure::Validation,
            CodeBlock::of("unreached", ()).unwrap(),
            vec![PipelineEvent::Validate(SpecOrigin::Failure)],
            "intentional validation failure",
        ),
        (
            PipelineSpecFailure::Emission,
            CodeBlock::of("unreached", ()).unwrap(),
            vec![
                PipelineEvent::Validate(SpecOrigin::Failure),
                PipelineEvent::Emit(SpecOrigin::Failure),
            ],
            "intentional emission failure",
        ),
        (
            PipelineSpecFailure::None,
            CodeBlock::of("INVALID_REWRITE", ()).unwrap(),
            vec![
                PipelineEvent::Validate(SpecOrigin::Failure),
                PipelineEvent::Emit(SpecOrigin::Failure),
                PipelineEvent::Rewrite(SourceLocation::Unlabelled),
            ],
            "rewritten source",
        ),
        (
            PipelineSpecFailure::None,
            CodeBlock::of("%T", (TypeName::primitive(" "),)).unwrap(),
            vec![
                PipelineEvent::Validate(SpecOrigin::Failure),
                PipelineEvent::Emit(SpecOrigin::Failure),
                PipelineEvent::Rewrite(SourceLocation::Unlabelled),
            ],
            "primitive spelling must not be blank",
        ),
        (
            PipelineSpecFailure::None,
            CodeBlock::of("%T", (TypeName::primitive("unsupported"),)).unwrap(),
            vec![
                PipelineEvent::Validate(SpecOrigin::Failure),
                PipelineEvent::Emit(SpecOrigin::Failure),
                PipelineEvent::Rewrite(SourceLocation::Unlabelled),
                PipelineEvent::Lower(TypeName::primitive("unsupported")),
            ],
            "intentional target rejection",
        ),
        (
            PipelineSpecFailure::None,
            CodeBlock::of("%T", (TypeName::primitive("invalid-lowering"),)).unwrap(),
            vec![
                PipelineEvent::Validate(SpecOrigin::Failure),
                PipelineEvent::Emit(SpecOrigin::Failure),
                PipelineEvent::Rewrite(SourceLocation::Unlabelled),
                PipelineEvent::Lower(TypeName::primitive("invalid-lowering")),
            ],
            "not valid inside one type expression",
        ),
    ];

    for (failure, block, expected_events, expected_error) in cases {
        let events = Rc::new(RefCell::new(Vec::new()));
        let file = FileSpec::builder_with("failure.pipeline", PipelineLang::new(events.clone()))
            .add_spec(PipelineSpec::one_block(
                events.clone(),
                SpecOrigin::Failure,
                block,
                failure,
            ))
            .build()
            .unwrap();

        let error = file.render(80).unwrap_err();

        assert!(
            error.to_string().contains(expected_error),
            "expected {expected_error:?}, got {error}"
        );
        assert_eq!(events.borrow().as_slice(), expected_events);
    }
}

#[test]
fn test_conflicting_imports() {
    let user1 = TypeName::importable_type("./models", "User");
    let user2 = TypeName::importable_type("./other", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("const u1: %T = get1()", (user1,));
    b.add_statement("const u2: %T = get2()", (user2,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("user.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.contains("const u1: User = get1();"));
    assert!(output.contains("const u2: OtherUser = get2();"));
    assert!(output.contains("User as OtherUser"));
}

#[test]
fn rewrite_effects_apply_at_every_structural_position_before_lowering_and_imports() {
    use ImportRewrite::{Introduce, Preserve, Remove, Replace};
    use SourcePosition::{Nested, Root, Sequence};

    for rewrite in [Preserve, Remove, Replace, Introduce] {
        for position in [Root, Nested, Sequence] {
            let events = Rc::new(RefCell::new(Vec::new()));
            let location = SourceLocation::ImportRewrite { rewrite, position };
            let file = FileSpec::builder_with(
                "rewrite-matrix.pipeline",
                PipelineLang::new(events.clone()),
            )
            .add_code(positioned_rewrite_block(rewrite, position))
            .build()
            .unwrap();

            let output = file.render(80).unwrap();
            let expected_type = expected_rewritten_type(rewrite);
            let mut expected_events = vec![PipelineEvent::Rewrite(location)];
            if let Some(type_name) = &expected_type {
                expected_events.push(PipelineEvent::Lower(type_name.clone()));
            }

            assert_eq!(
                events.borrow().as_slice(),
                expected_events,
                "rewrite={rewrite:?}, position={position:?}"
            );

            let import_lines: Vec<_> = output
                .lines()
                .filter(|line| line.starts_with("import "))
                .collect();
            let expected_imports = match rewrite {
                Preserve => vec!["import Preserved as Preserved from ./preserved"],
                Remove => Vec::new(),
                Replace => vec!["import Replaced as Replaced from ./replacement"],
                Introduce => vec!["import User as User from ./rewritten"],
            };
            assert_eq!(
                import_lines, expected_imports,
                "rewrite={rewrite:?}, position={position:?}"
            );

            match expected_type {
                Some(TypeName::Importable { name, .. }) => assert!(
                    output.lines().any(|line| line == name),
                    "rewritten type was not rendered for rewrite={rewrite:?}, position={position:?}: {output:?}"
                ),
                None => assert!(
                    !output.contains("Removed") && !output.contains("./removed"),
                    "removed type survived for position={position:?}: {output:?}"
                ),
                Some(other) => panic!("unexpected rewrite expectation: {other:?}"),
            }
        }
    }
}

#[test]
fn source_block_owners_are_rewritten_and_lowered_in_dispatch_order() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let header_type = TypeName::importable_type("./header", "HeaderType");
    let stored_type = TypeName::importable_type("./stored", "StoredType");
    let single_type = TypeName::importable_type("./single", "SingleType");
    let multiple_zero_type = TypeName::importable_type("./multiple-zero", "MultipleZeroType");
    let multiple_one_type = TypeName::importable_type("./multiple-one", "MultipleOneType");

    let file = FileSpec::builder_with("owners.pipeline", PipelineLang::new(events.clone()))
        .header(located_type(SourceLocation::Header, header_type.clone()))
        .add_code(located_type(
            SourceLocation::StoredCode,
            stored_type.clone(),
        ))
        .add_spec(PipelineSpec::one_block(
            events.clone(),
            SpecOrigin::SingleBlock,
            located_type(SourceLocation::MaterializedSingle, single_type.clone()),
            PipelineSpecFailure::None,
        ))
        .add_spec(PipelineSpec::multiple_blocks(
            events.clone(),
            SpecOrigin::MultipleBlocks,
            vec![
                located_type(
                    SourceLocation::MaterializedMultiple(0),
                    multiple_zero_type.clone(),
                ),
                located_type(
                    SourceLocation::MaterializedMultiple(1),
                    multiple_one_type.clone(),
                ),
            ],
        ))
        .build()
        .unwrap();

    let output = file.render(80).unwrap();

    assert_eq!(
        events.borrow().as_slice(),
        &[
            PipelineEvent::Validate(SpecOrigin::SingleBlock),
            PipelineEvent::Validate(SpecOrigin::MultipleBlocks),
            PipelineEvent::Emit(SpecOrigin::SingleBlock),
            PipelineEvent::Emit(SpecOrigin::MultipleBlocks),
            PipelineEvent::Rewrite(SourceLocation::Header),
            PipelineEvent::Lower(header_type),
            PipelineEvent::Rewrite(SourceLocation::StoredCode),
            PipelineEvent::Lower(stored_type),
            PipelineEvent::Rewrite(SourceLocation::MaterializedSingle),
            PipelineEvent::Lower(single_type),
            PipelineEvent::Rewrite(SourceLocation::MaterializedMultiple(0)),
            PipelineEvent::Lower(multiple_zero_type),
            PipelineEvent::Rewrite(SourceLocation::MaterializedMultiple(1)),
            PipelineEvent::Lower(multiple_one_type),
        ]
    );
    assert_eq!(
        output
            .lines()
            .filter(|line| line.starts_with("import "))
            .collect::<Vec<_>>(),
        [
            "import HeaderType as HeaderType from ./header",
            "import StoredType as StoredType from ./stored",
            "import SingleType as SingleType from ./single",
            "import MultipleZeroType as MultipleZeroType from ./multiple-zero",
            "import MultipleOneType as MultipleOneType from ./multiple-one",
        ]
    );
    for rendered_type in [
        "HeaderType",
        "StoredType",
        "SingleType",
        "MultipleZeroType",
        "MultipleOneType",
    ] {
        assert!(
            output.lines().any(|line| line == rendered_type),
            "missing {rendered_type} in {output:?}"
        );
    }
}

#[test]
fn raw_import_metadata_matrix_lowers_without_rewriting_opaque_content() {
    let content = "INSERT_IMPORT __source_header__\r\n";
    let compound = TypeName::generic(
        TypeName::primitive("MetadataCompound"),
        vec![TypeName::primitive("Value")],
    );
    let cases = [
        (TypeName::primitive("MetadataPrimitive"), None),
        (
            TypeName::importable_type("./metadata", "MetadataImport"),
            Some("import MetadataImport as MetadataImport from ./metadata"),
        ),
        (
            compound,
            Some("import MetadataCompound as MetadataCompound from ./metadata-compound"),
        ),
        (
            TypeName::primitive("MetadataDerived"),
            Some("import MetadataDerived as MetadataDerived from ./metadata-derived"),
        ),
    ];

    for (metadata_type, expected_import) in cases {
        let events = Rc::new(RefCell::new(Vec::new()));
        let file = FileSpec::builder_with("raw.pipeline", PipelineLang::new(events.clone()))
            .add_raw_with_imports(content, vec![metadata_type.clone()])
            .build()
            .unwrap();

        let output = file.render(80).unwrap();

        assert!(
            output.ends_with(content),
            "opaque bytes changed for {metadata_type:?}: {output:?}"
        );
        assert_eq!(
            events.borrow().as_slice(),
            &[PipelineEvent::Lower(metadata_type.clone())]
        );
        assert_eq!(
            output
                .lines()
                .filter(|line| line.starts_with("import "))
                .collect::<Vec<_>>(),
            expected_import.into_iter().collect::<Vec<_>>()
        );
    }
}

#[test]
fn invalid_raw_import_metadata_fails_before_lowering_or_rewriting() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let file = FileSpec::builder_with("raw-invalid.pipeline", PipelineLang::new(events.clone()))
        .add_raw_with_imports(
            "INSERT_IMPORT __source_header__\r\n",
            vec![TypeName::primitive(" ")],
        )
        .build()
        .unwrap();

    assert!(matches!(
        file.render(80),
        Err(SigilStitchError::InvalidTypeName { reason, .. })
            if reason == "primitive spelling must not be blank"
    ));
    assert!(events.borrow().is_empty());
}

#[test]
fn repeated_structural_types_are_lowered_once_per_original_root() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let repeated = TypeName::generic(
        TypeName::primitive("Container"),
        vec![TypeName::primitive("Value")],
    );
    let block = CodeBlock::of("%T and %T", (repeated.clone(), repeated.clone())).unwrap();
    let file = FileSpec::builder_with("repeated.pipeline", PipelineLang::new(events.clone()))
        .add_code(block)
        .build()
        .unwrap();

    assert!(
        file.render(80)
            .unwrap()
            .contains("LoweredType and LoweredType")
    );
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|event| **event == PipelineEvent::Lower(repeated.clone()))
            .count(),
        2
    );
}

#[test]
fn invalid_type_lowering_output_aborts_before_source_rendering() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let file = FileSpec::builder_with("invalid.pipeline", PipelineLang::new(events.clone()))
        .add_code(
            CodeBlock::of(
                "prefix %T suffix",
                (TypeName::primitive("invalid-lowering"),),
            )
            .unwrap(),
        )
        .build()
        .unwrap();

    assert!(matches!(
        file.render(80),
        Err(SigilStitchError::InvalidTypeNameLowering { reason, .. })
            if reason.contains("not valid inside one type expression")
    ));
    assert_eq!(
        events.borrow().last(),
        Some(&PipelineEvent::Lower(TypeName::primitive(
            "invalid-lowering"
        )))
    );
}

#[test]
fn file_render_uses_custom_alias_assignments_or_returns_only_an_error() {
    let block = CodeBlock::of(
        "%T %T",
        (
            TypeName::importable_type("./models", "User"),
            TypeName::importable_type("./other", "User"),
        ),
    )
    .unwrap();
    let file = FileSpec::builder("custom.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file
        .render_with_import_alias_resolver(80, &ModuleResolver)
        .unwrap();
    assert!(output.contains("User as PrimaryUser"));
    assert!(output.contains("User as SecondaryUser"));
    assert!(output.contains("PrimaryUser SecondaryUser"));

    assert!(matches!(
        file.render_with_import_alias_resolver(80, &RejectingResolver),
        Err(SigilStitchError::ImportAliasResolverRejected { reason })
            if reason == "aliases disabled by caller"
    ));
}

#[test]
fn standalone_renderer_uses_the_callers_resolved_imports_without_mutating_them() {
    let imports = ImportGroup::from(vec![ImportEntry {
        module: "./models".to_string(),
        name: "User".to_string(),
        alias: Some("CallerUser".to_string()),
        is_type_only: true,
        is_side_effect: false,
        is_wildcard: false,
    }]);
    let block = CodeBlock::of(
        "value: %T",
        (TypeName::importable_type("./models", "User"),),
    )
    .unwrap();
    let lang = sigil_stitch::lang::typescript::TypeScript::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(&lang, &imports, 80);

    assert_eq!(renderer.render(&block).unwrap(), "value: CallerUser");
    assert_eq!(imports.entries()[0].alias.as_deref(), Some("CallerUser"));
}

#[test]
fn test_raw_content_no_import_tracking() {
    let file = FileSpec::builder("raw.ts")
        .add_raw("// This is raw content\nexport const VERSION = '1.0.0';\n")
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.contains("// This is raw content"));
    assert!(output.contains("export const VERSION = '1.0.0';"));
    assert!(!output.contains("import"));
}

#[test]
fn test_mixed_code_and_raw() {
    let user = TypeName::importable_type("./models", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("const u: %T = getUser()", (user,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("mixed.ts")
        .add_raw("// Generated file, do not edit.\n")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.contains("import type { User }"));
    assert!(output.contains("// Generated file"));
    assert!(output.contains("const u: User = getUser();"));
}

#[test]
fn test_file_with_header() {
    let mut header_builder = CodeBlock::builder();
    header_builder.add("// License: MIT", ());
    let header = header_builder.build().unwrap();

    let mut b = CodeBlock::builder();
    b.add_statement("const x = 1", ());
    let block = b.build().unwrap();

    let file = FileSpec::builder("test.ts")
        .header(header)
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.starts_with("// License: MIT"));
    assert!(output.contains("const x = 1;"));
}

#[test]
fn test_dedup_same_import() {
    let user1 = TypeName::importable_type("./models", "User");
    let user2 = TypeName::importable_type("./models", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("const u1: %T = get1()", (user1,));
    b.add_statement("const u2: %T = get2()", (user2,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("user.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    let import_count = output.matches("import type { User }").count();
    assert_eq!(import_count, 1);
}

#[test]
fn test_build_empty_filename_errors() {
    let result = FileSpec::builder("").build();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("'name' must not be empty")
    );
}

#[test]
fn test_aliased_type_in_codeblock() {
    let user = TypeName::importable("./models", "User").with_alias("UserModel");

    let mut b = CodeBlock::builder();
    b.add_statement("const u: %T = getUser()", (user,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("user.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(
        output.contains("User as UserModel"),
        "Expected aliased import, got:\n{output}"
    );
    assert!(
        output.contains("const u: UserModel = getUser();"),
        "Expected alias in code, got:\n{output}"
    );
}

#[test]
fn test_aliased_type_with_auto_alias_conflict() {
    let user1 = TypeName::importable_type("./models", "User").with_alias("ModelUser");
    let user2 = TypeName::importable_type("./other", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("const u1: %T = get1()", (user1,));
    b.add_statement("const u2: %T = get2()", (user2,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("user.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(
        output.contains("const u1: ModelUser = get1();"),
        "Expected preferred alias, got:\n{output}"
    );
    assert!(
        output.contains("const u2: OtherUser = get2();"),
        "Expected auto-alias for second, got:\n{output}"
    );
}

#[test]
fn test_serde_round_trip_render_returns_error_without_lang() {
    let file = FileSpec::builder("test.ts")
        .add_code(CodeBlock::of("const x = 1", ()).unwrap())
        .build()
        .unwrap();

    let json = serde_json::to_string(&file).unwrap();
    let deserialized: FileSpec = serde_json::from_str(&json).unwrap();

    let err = deserialized.render(80).unwrap_err();
    assert!(err.to_string().contains("no language"));
}

#[test]
fn test_serde_round_trip_with_lang() {
    use sigil_stitch::lang::typescript::TypeScript;

    let mut b = CodeBlock::builder();
    b.add_statement("const x = 1", ());
    let file = FileSpec::builder("test.ts")
        .add_code(b.build().unwrap())
        .build()
        .unwrap();

    let json = serde_json::to_string(&file).unwrap();
    let deserialized: FileSpec = serde_json::from_str(&json).unwrap();

    let output = deserialized
        .with_lang(TypeScript::new())
        .render(80)
        .unwrap();
    assert!(
        output.contains("const x = 1;"),
        "Expected 'const x = 1;' in output:\n{output}"
    );
}

#[test]
fn test_custom_emittable_via_add_spec() {
    #[derive(Debug)]
    struct CommentSpec(&'static str);

    impl Emittable for CommentSpec {
        fn emit_members(&self, lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
            let mut cb = CodeBlock::builder();
            let p = lang.line_comment_prefix();
            let s = lang.line_comment_suffix();
            cb.add(&format!("{p} {}{s}", self.0), ());
            Ok(vec![cb.build()?])
        }
    }

    let mut code_cb = CodeBlock::builder();
    code_cb.add_statement("const x = 1", ());

    let file = FileSpec::builder("test.ts")
        .add_code(code_cb.build().unwrap())
        .add_spec(CommentSpec("AUTO-GENERATED"))
        .add_function(
            FunSpec::builder("foo")
                .body(CodeBlock::of("return", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.contains("const x = 1;"), "code member: {output}");
    assert!(
        output.contains("// AUTO-GENERATED"),
        "spec member: {output}"
    );
    assert!(output.contains("function foo()"), "fun member: {output}");
}

#[test]
fn test_spec_with_imports() {
    #[derive(Debug)]
    struct TypedConstSpec(TypeName);

    impl Emittable for TypedConstSpec {
        fn emit_members(&self, _lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
            let mut cb = CodeBlock::builder();
            cb.add_statement("const current: %T = null", (self.0.clone(),));
            Ok(vec![cb.build()?])
        }
    }

    let file = FileSpec::builder("test.ts")
        .add_spec(TypedConstSpec(TypeName::importable_type(
            "./models", "User",
        )))
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(
        output.contains("import type { User }"),
        "import should be collected from Spec member: {output}"
    );
    assert!(
        output.contains("const current: User = null;"),
        "body: {output}"
    );
}

#[test]
fn test_spec_error_propagation() {
    #[derive(Debug)]
    struct FailingSpec;

    impl Emittable for FailingSpec {
        fn emit_members(&self, _lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
            Err(SigilStitchError::Render {
                context: "FailingSpec".into(),
                message: "intentional test error".into(),
            })
        }
    }

    let file = FileSpec::builder("test.ts")
        .add_spec(FailingSpec)
        .build()
        .unwrap();

    let err = file.render(80).unwrap_err();
    assert!(err.to_string().contains("intentional test error"), "{err}");
}

#[derive(Debug, Clone, Copy)]
enum FailingHook {
    Newtype,
    Context,
    Suffix,
}

#[derive(Debug)]
struct FailingHookLang(FailingHook);

impl RendererLang for FailingHookLang {
    fn file_extension(&self) -> &str {
        "fail"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for FailingHookLang {
    fn emit_newtype_decl(
        &self,
        _visibility: &str,
        name: &str,
        _type_params: &[TypeParamSpec],
        inner: &TypeName,
    ) -> Result<CodeBlock, SigilStitchError> {
        if matches!(self.0, FailingHook::Newtype) {
            return CodeBlock::of("%T %T", inner.clone());
        }
        CodeBlock::of(&format!("struct {name}(%T);"), inner.clone())
    }

    fn emit_type_context(
        &self,
        _type_params: &[TypeParamSpec],
    ) -> Result<Option<CodeBlock>, SigilStitchError> {
        if matches!(self.0, FailingHook::Context) {
            return Err(SigilStitchError::Render {
                context: "emit_type_context".into(),
                message: "intentional hook error".into(),
            });
        }
        Ok(None)
    }

    fn emit_type_close_suffix(
        &self,
        _kind: TypeKind,
        _impl_types: &[TypeName],
    ) -> Result<Option<CodeBlock>, SigilStitchError> {
        if matches!(self.0, FailingHook::Suffix) {
            return Err(SigilStitchError::Render {
                context: "emit_type_close_suffix".into(),
                message: "intentional hook error".into(),
            });
        }
        Ok(None)
    }

    #[allow(deprecated)] // Exercises the frozen 0.6.8 compatibility lowerer.
    fn function_syntax(&self) -> sigil_stitch::lang::config::FunctionSyntaxConfig<'_> {
        sigil_stitch::lang::config::FunctionSyntaxConfig {
            function_signature_style: sigil_stitch::spec::fun_spec::FunctionSignatureStyle::Split,
            ..Default::default()
        }
    }
}

#[test]
fn test_structured_hook_errors_propagate_from_file_render() {
    let newtype = TypeSpec::builder("Wrapped", TypeKind::Newtype)
        .extends(TypeName::primitive("String"))
        .build()
        .unwrap();
    let newtype_error =
        FileSpec::builder_with("wrapped.fail", FailingHookLang(FailingHook::Newtype))
            .add_type(newtype)
            .build()
            .unwrap()
            .render(80)
            .unwrap_err();
    assert!(
        newtype_error
            .to_string()
            .contains("expects 2 args but got 1"),
        "{newtype_error}"
    );

    let function = FunSpec::builder("display")
        .add_type_param(TypeParamSpec::new("T"))
        .add_param(ParameterSpec::new("value", TypeName::primitive("T")).unwrap())
        .returns(TypeName::primitive("String"))
        .build()
        .unwrap();
    let context_error =
        FileSpec::builder_with("display.fail", FailingHookLang(FailingHook::Context))
            .add_function(function)
            .build()
            .unwrap()
            .render(80)
            .unwrap_err();
    assert!(
        context_error.to_string().contains("emit_type_context"),
        "{context_error}"
    );

    let type_spec = TypeSpec::builder("Record", TypeKind::Struct)
        .build()
        .unwrap();
    let suffix_error = FileSpec::builder_with("record.fail", FailingHookLang(FailingHook::Suffix))
        .add_type(type_spec)
        .build()
        .unwrap()
        .render(80)
        .unwrap_err();
    assert!(
        suffix_error.to_string().contains("emit_type_close_suffix"),
        "{suffix_error}"
    );
}

#[test]
fn test_serde_rejects_spec_variant() {
    #[derive(Debug)]
    struct Dummy;

    impl Emittable for Dummy {
        fn emit_members(&self, _lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
            Ok(vec![])
        }
    }

    let file = FileSpec::builder("test.ts")
        .add_spec(Dummy)
        .build()
        .unwrap();

    let err = serde_json::to_string(&file).unwrap_err();
    assert!(err.to_string().contains("cannot be serialized"), "{err}");
}

#[test]
fn validate_reports_unsupported_type_before_render() {
    let file = FileSpec::builder_with("user.bash", sigil_stitch::lang::bash::Bash::new())
        .add_type(TypeSpec::builder("User", TypeKind::Class).build().unwrap())
        .build()
        .unwrap();

    for error in [file.validate().unwrap_err(), file.render(80).unwrap_err()] {
        let SigilStitchError::FileSpecValidation {
            filename,
            error_count,
            errors,
        } = error
        else {
            panic!("expected FileSpecValidation, got {error:?}");
        };
        assert_eq!(filename, "user.bash");
        assert_eq!(error_count, 1);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            SigilStitchError::UnsupportedTypeKind { type_name, .. } if type_name == "User"
        ));
    }
}

#[test]
fn validate_aggregates_multiple_unsupported_types() {
    let file = FileSpec::builder_with("user.bash", sigil_stitch::lang::bash::Bash::new())
        .add_type(TypeSpec::builder("User", TypeKind::Class).build().unwrap())
        .add_type(
            TypeSpec::builder("Account", TypeKind::Class)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let error = file.validate().unwrap_err();
    let SigilStitchError::FileSpecValidation {
        filename,
        error_count,
        errors,
    } = error
    else {
        panic!("expected FileSpecValidation, got {error:?}");
    };

    assert_eq!(filename, "user.bash");
    assert_eq!(error_count, 2);
    assert_eq!(errors.len(), 2);

    let mut names: Vec<_> = errors
        .iter()
        .map(|error| match error {
            SigilStitchError::UnsupportedTypeKind { type_name, .. } => type_name.as_str(),
            other => panic!("expected UnsupportedTypeKind, got {other:?}"),
        })
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["Account", "User"]);

    let render_error = file.render(80).unwrap_err();
    assert!(matches!(
        render_error,
        SigilStitchError::FileSpecValidation { .. }
    ));
}

#[test]
fn add_spec_cannot_bypass_builtin_aggregate_validation() {
    let invalid_fun = FunSpec::builder("work").is_async().build().unwrap();
    let file = FileSpec::builder_with("mixed.bash", sigil_stitch::lang::bash::Bash::new())
        .add_spec(TypeSpec::builder("User", TypeKind::Class).build().unwrap())
        .add_spec(invalid_fun.clone())
        .add_function(invalid_fun)
        .build()
        .unwrap();

    let SigilStitchError::FileSpecValidation {
        error_count,
        errors,
        ..
    } = file.validate().unwrap_err()
    else {
        panic!("expected FileSpecValidation");
    };

    assert_eq!(error_count, 3);
    assert!(matches!(
        errors[0],
        SigilStitchError::UnsupportedTypeKind { .. }
    ));
    assert!(errors[1..].iter().all(|error| matches!(
        error,
        SigilStitchError::UnsupportedFunctionCapabilities { .. }
    )));
}

#[test]
fn validate_missing_lang_stays_direct() {
    let file = FileSpec::builder("empty.ts").build().unwrap();
    let json = serde_json::to_string(&file).unwrap();
    let deserialized: FileSpec = serde_json::from_str(&json).unwrap();

    let error = deserialized.validate().unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::MissingLang { ref filename } if filename == "empty.ts"
    ));
}

#[test]
fn legacy_adapter_defaults_to_permissive_capabilities() {
    let file = FileSpec::builder_with("wrapped.fail", FailingHookLang(FailingHook::Newtype))
        .add_type(
            TypeSpec::builder("Wrapper", TypeKind::Newtype)
                .extends(TypeName::primitive("String"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    assert!(file.validate().is_ok());
}

#[test]
fn legacy_adapter_defaults_to_permissive_function_capabilities() {
    let fun = FunSpec::builder("work")
        .is_async()
        .body(CodeBlock::of("return 1", ()).unwrap())
        .build()
        .unwrap();
    let file = FileSpec::builder_with("work.fail", FailingHookLang(FailingHook::Newtype))
        .add_function(fun)
        .build()
        .unwrap();

    assert!(file.validate().is_ok());
}
