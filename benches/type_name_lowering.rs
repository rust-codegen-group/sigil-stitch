use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::code_node::CodeNode;
use sigil_stitch::code_renderer::CodeRenderer;
use sigil_stitch::import::ImportGroup;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::type_name::TypeName;

fn structural_type(index: usize) -> TypeName {
    TypeName::generic(
        TypeName::primitive(&format!("Container{index}")),
        vec![
            TypeName::array(TypeName::primitive(&format!("Value{index}"))),
            TypeName::optional(TypeName::primitive(&format!("Error{index}"))),
        ],
    )
}

fn wide_block(type_count: usize) -> CodeBlock {
    let mut block = CodeBlock::of("", ()).unwrap();
    for index in 0..type_count {
        if index > 0 {
            block.nodes_mut().push(CodeNode::Literal("; ".to_string()));
        }
        block
            .nodes_mut()
            .push(CodeNode::TypeRef(structural_type(index)));
    }
    block
}

fn nested_chunk(start: usize, type_count: usize, depth: usize) -> CodeBlock {
    if depth == 0 {
        let mut block = CodeBlock::of("", ()).unwrap();
        let mut sequence = Vec::with_capacity(type_count.saturating_mul(2));
        for index in start..start + type_count {
            if index > start {
                sequence.push(CodeNode::Literal("; ".to_string()));
            }
            sequence.push(CodeNode::TypeRef(structural_type(index)));
        }
        block.nodes_mut().push(CodeNode::Sequence(sequence));
        return block;
    }

    CodeBlock::of("%L", nested_chunk(start, type_count, depth - 1)).unwrap()
}

fn nested_block(type_count: usize) -> CodeBlock {
    const CHUNK_SIZE: usize = 16;
    let mut block = CodeBlock::of("", ()).unwrap();
    for start in (0..type_count).step_by(CHUNK_SIZE) {
        let chunk_size = CHUNK_SIZE.min(type_count - start);
        block
            .nodes_mut()
            .push(CodeNode::Nested(nested_chunk(start, chunk_size, 3)));
    }
    block
}

fn render(block: &CodeBlock) {
    let language = TypeScript::new();
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(&language, &imports, usize::MAX);
    black_box(renderer.render(black_box(block)).unwrap());
}

fn benchmark_type_name_lowering(criterion: &mut Criterion) {
    const SIZES: [usize; 3] = [128, 256, 512];

    let mut wide = criterion.benchmark_group("type_name_lowering_wide");
    for size in SIZES {
        let input = wide_block(size);
        wide.throughput(Throughput::Elements(size as u64));
        wide.bench_with_input(
            BenchmarkId::from_parameter(size),
            &input,
            |bencher, block| {
                bencher.iter(|| render(block));
            },
        );
    }
    wide.finish();

    let mut nested = criterion.benchmark_group("type_name_lowering_nested");
    for size in SIZES {
        let input = nested_block(size);
        nested.throughput(Throughput::Elements(size as u64));
        nested.bench_with_input(
            BenchmarkId::from_parameter(size),
            &input,
            |bencher, block| {
                bencher.iter(|| render(block));
            },
        );
    }
    nested.finish();
}

criterion_group!(benches, benchmark_type_name_lowering);
criterion_main!(benches);
