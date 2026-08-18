use syn::{GenericArgument, PathArguments, Type};

pub(crate) fn is_secret_type(ty: &Type) -> bool {
    matches!(last_type_ident(ty).as_deref(), Some("Secret"))
}

pub(crate) fn metadata_target_type(ty: &Type) -> &Type {
    let Some(inner) = metadata_inner_type(ty) else {
        return ty;
    };
    metadata_target_type(inner)
}

pub(crate) fn metadata_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    match segment.ident.to_string().as_str() {
        "Option" | "Box" | "Arc" => match &segment.arguments {
            PathArguments::AngleBracketed(arguments) => {
                arguments.args.iter().find_map(|argument| {
                    if let GenericArgument::Type(ty) = argument {
                        Some(ty)
                    } else {
                        None
                    }
                })
            }
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn option_inner_type(ty: &Type) -> Option<&Type> {
    wrapper_inner_type(ty, "Option")
}

pub(crate) fn patch_inner_type(ty: &Type) -> Option<&Type> {
    wrapper_inner_type(ty, "Patch")
}

fn wrapper_inner_type<'a>(ty: &'a Type, wrapper: &str) -> Option<&'a Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != wrapper {
        return None;
    }
    match &segment.arguments {
        PathArguments::AngleBracketed(arguments) => arguments.args.iter().find_map(|argument| {
            if let GenericArgument::Type(ty) = argument {
                Some(ty)
            } else {
                None
            }
        }),
        _ => None,
    }
}

fn last_type_ident(ty: &Type) -> Option<String> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    type_path
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

pub(crate) fn supports_append_strategy(ty: &Type) -> bool {
    let Some(inner) = metadata_inner_type(ty) else {
        return matches!(ty, Type::Array(_))
            || matches!(last_type_ident(ty).as_deref(), Some("Vec"));
    };
    supports_append_strategy(inner)
}
