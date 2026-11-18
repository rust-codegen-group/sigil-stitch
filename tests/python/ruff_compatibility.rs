use std::process::{Command, Stdio};

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::python::Python;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::type_name::TypeName;

fn render(block: &CodeBlock, width: usize) -> String {
    FileSpec::builder("generated.py")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(width)
        .unwrap()
}

fn ruff_command() -> Command {
    if std::env::var_os("SIGIL_STITCH_RUFF_UVX").is_some() {
        let mut command = Command::new("uvx");
        command.args(["--from", "ruff==0.17.5", "ruff"]);
        command
    } else {
        Command::new("ruff")
    }
}

fn assert_ruff_fixed_point(source: &str, filename: &str, width: usize) {
    let version = ruff_command()
        .arg("--version")
        .output()
        .expect("the pinned Ruff oracle must be installed");
    assert!(
        version.status.success(),
        "Ruff --version failed: {version:?}"
    );
    let version = String::from_utf8_lossy(&version.stdout);
    assert_eq!(
        version.trim(),
        "ruff 0.17.5",
        "unexpected Ruff oracle version"
    );

    let mut command = ruff_command();
    let mut child = command
        .args([
            "format",
            "--check",
            "--diff",
            "--isolated",
            "--line-length",
            &width.to_string(),
            "--stdin-filename",
            filename,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the pinned Ruff oracle must be runnable");
    let mut stdin = child
        .stdin
        .take()
        .expect("Ruff stdin should be available for the fixed-point check");
    std::io::Write::write_all(&mut stdin, source.as_bytes()).expect("write source to Ruff");
    drop(stdin);
    let output = child
        .wait_with_output()
        .expect("wait for Ruff fixed-point check");
    assert!(
        output.status.success(),
        "Ruff changed {filename} at width {width}:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires the pinned Ruff 0.17.5 oracle; run scripts/check-ruff-python.sh"]
fn generated_python_is_ruff_fixed_point() {
    let generator = sigil_quote!(Python {
        if (_support.is_dict(value) and all((_support.number(_item, True)
            or isinstance(_item, str)) for _item in _support.as_dict(value).values())) {
            return value;
        }
    })
    .unwrap();
    let rendered = render(&generator, 88);
    assert_ruff_fixed_point(&rendered, "generator.py", 88);

    let local_import = sigil_quote!(Python {
        def load(): {
            from . import sibling
            return sibling
        }
    })
    .unwrap();
    assert_ruff_fixed_point(&render(&local_import, 88), "local_import.py", 88);

    let comprehension = sigil_quote!(Python {
        return {key: convert(item) for key, item in values.items()}
    })
    .unwrap();
    for width in [40, 88] {
        assert_ruff_fixed_point(&render(&comprehension, width), "comprehension.py", width);
    }

    let initializer = sigil_quote!(Python {
        additional_properties: _dict[_str, dict[str, Choice] | str] = _dataclass_field(default_factory=lambda: _dict[_str, dict[str, Choice] | str](), kw_only=True, metadata={"openapi_additional_properties": True, "openapi_dictionary": True})
    })
    .unwrap();
    assert_ruff_fixed_point(&render(&initializer, 88), "initializer.py", 88);

    let semantic_body = sigil_quote!(Python {
        return {key: convert(item) for key, item in values.items()}
    })
    .unwrap();
    let semantic_function = FunSpec::builder("convert_values")
        .add_param(ParameterSpec::new("values", TypeName::primitive("object")).unwrap())
        .returns(vec![TypeName::primitive("object")])
        .body(semantic_body)
        .build()
        .unwrap();
    let semantic_file = FileSpec::builder_with("semantic.py", Python::new())
        .add_function(semantic_function)
        .build()
        .unwrap()
        .render(88)
        .unwrap();
    assert_ruff_fixed_point(&semantic_file, "semantic.py", 88);

    let first = sigil_quote!(Python {
        def first(): {
            pass
        }
    })
    .unwrap();
    let second = sigil_quote!(Python {
        def second(): {
            pass
        }
    })
    .unwrap();
    let file = FileSpec::builder("declarations.py")
        .add_code(first)
        .add_code(second)
        .build()
        .unwrap()
        .render(88)
        .unwrap();
    assert_ruff_fixed_point(&file, "declarations.py", 88);
}
