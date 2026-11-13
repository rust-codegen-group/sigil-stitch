# Floating Values

`FloatValue` describes a machine floating value by its scalar format and exact
bit pattern. It is distinct from a `TypeName`, a numeric source token, or a
request to round a decimal number.

## Construct and Compose

The two current variants carry integer bits:

- `FloatValue::Binary32(u32)` is IEEE 754 binary32.
- `FloatValue::Binary64(u64)` is IEEE 754 binary64.

`from_f32` and `from_f64` capture an existing Rust value's bits without numeric
conversion. If that host value was already rounded from a decimal literal, these
constructors preserve the existing machine value, not the original decimal text.

```rust
# extern crate sigil_stitch;
use sigil_stitch::lang::rust::Rust;
use sigil_stitch::prelude::*;

# fn main() -> Result<(), SigilStitchError> {
let negative_zero = FloatValue::from_f64(-0.0);
assert_eq!(negative_zero, FloatValue::Binary64(0x8000_0000_0000_0000));

let body = CodeBlock::of(
    "const NEGATIVE_ZERO: %T = %L;",
    (TypeName::primitive("f64"), negative_zero),
)?;
let output = FileSpec::builder_with("values.rs", Rust::new())
    .add_code(body)
    .build()?
    .render(80)?;
assert!(output.contains("::core::primitive::f64::from_bits(0x8000000000000000u64)"));
# Ok(())
# }
```

Pass the value directly through `%L`, macro `$L`, or a template's `L` parameter.
These existing composition paths store a `CodeNode::FloatValue` inside a nested
block; they do not select a language or produce floating-point source text.
The selected file or renderer adapter constructs the expression during source
preparation. There is no separate public `emit` call or new format character.

The macro form is equally direct:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::lang::rust::Rust;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), SigilStitchError> {
let body = sigil_quote!(Rust {
    const GAIN: $T(TypeName::primitive("f32")) = $L(FloatValue::from_f32(1.0));
})?;
assert!(body.render_standalone(&Rust::new(), 80)?.contains("from_bits(0x3f800000u32)"));
# Ok(())
# }
```

The macro's language tag controls tokenization, not floating-value construction.
That operation uses the actual file or renderer adapter. The value's format and
bits remain independent of the target, but surrounding raw source such as
`const GAIN: f32` is still Rust syntax: this does not make an entire `CodeBlock`
a portable program. `$C` remains for existing blocks, not bare floating values.
Passing a bare host `f64` to `%L` does not implicitly capture its bits; construct
a `FloatValue` explicitly.

## Preservation and Support

Rust is currently the only built-in adapter that supports exact construction.
It emits absolute `::core::primitive::f32::from_bits` or
`::core::primitive::f64::from_bits` calls with typed integer arguments. These
work in `no_std` source and const contexts, and local names do not capture the
primitive paths. All bit patterns are supported, including subnormals,
infinities, both signed zeros, and different quiet/signaling NaN encodings.

The other built-in adapters and unchanged external adapters return
`SigilStitchError::UnsupportedFloatValue` when preparing the block for rendering.
Building a block or template does not require that a target is already selected.
Unsupported targets do not emit an approximate number,
change width, return a byte array, or fall back to a decimal string.

Preservation applies to constructing the requested value. Later target-language
arithmetic may change a NaN payload or a signed zero; no arithmetic behavior is
promised by this interface. Equality and hashing compare format and bits rather
than IEEE numeric equality: positive and negative zero differ, while a NaN
encoding equals itself. Derived Serde support permits same-version round-trips,
not a versioned wire or binary format. The same applies to source blocks that
contain `CodeNode::FloatValue`.

BF16, FP8 variants, shared-scale MX formats, quantization and decimal-to-format
conversion are not currently supported. Floating singleton types are also a
separate concept; `TypeName::IntegerLiteral` remains unchanged.

## Language-Owned Construction

`RendererLang::lower_float_value(&FloatValue)` is the language-owned operation.
Its provided implementation rejects the request, so existing adapters need not
implement a new required trait method. An override returns a non-empty,
structurally balanced expression block or propagates a `SigilStitchError`.
It must preserve the format and bits, keep type references structured, and return
a complete expression rather than another deferred floating-value node.

Standalone and file rendering share the same source preparer. It rewrites and
validates the caller's source tree, then lowers floating-value nodes through the
selected adapter, recursively through nested blocks and sequences. Empty,
layout-only, structurally malformed, or still-deferred results fail with
`InvalidFloatValueLowering`; original hook failures propagate unchanged.

Type references introduced by floating-value construction then pass through
ordinary type-name lowering and validation before import collection and alias
resolution. Completed lowering results are not source-rewritten again. Only
fully prepared source reaches the final renderer, including any soft breaks
introduced by construction. Preparation does not mutate the original block.
The adapter owns target-expression grammar; structural validation is not a
general target-language parser or type checker.
