#![deny(deprecated)]

use sigil_stitch::prelude::*;

fn main() {
    let value = TypeName::primitive("Value");
    let _ = TypeName::generic(value.clone(), vec![value.clone()]);
    let _ = TypeName::Generic {
        base: Box::new(value.clone()),
        params: vec![value.clone()],
    };
    let _ = TypeName::function(vec![value.clone()], value.clone());
    let _ = TypeName::Function {
        params: vec![value.clone()],
        return_type: Box::new(value),
    };
    let _ = TypeParamSpec::new("T");
    let _ = TypeParamKind::Constructor1;
    let _ = FunSpec::builder("work").add_type_param(TypeParamSpec::new("T"));
    let _ = TypeSpec::builder("Owner", TypeKind::Class)
        .add_type_param(TypeParamSpec::new("T"));
}
