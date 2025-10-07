use std::num::NonZero;

use simplicityhl::{
    num::NonZeroPow2Usize,
    parse::CallName,
    str::{AliasName, FunctionName},
    types::AliasedType,
};

use crate::completion::types::FunctionTemplate;

/// Macro to convert string literals into `Vec<String>`
macro_rules! str_vec {
    () => {
        Vec::new()
    };
    ($($item:expr),+ $(,)?) => {
        vec![$($item.to_string()),+]
    };
}

// TODO: More verbose descriptions with examples

/// Get completion of builtin functions. They are all defined in `simplicityhl::parse::CallName`
pub fn get_builtin_functions() -> Vec<FunctionTemplate> {
    let ty = AliasedType::from(AliasName::from_str_unchecked("T"));
    let Some(some) = NonZero::new(1) else {
        return vec![];
    };

    let functions = vec![
        CallName::UnwrapLeft(ty.clone()),
        CallName::UnwrapRight(ty.clone()),
        CallName::Unwrap,
        CallName::IsNone(ty.clone()),
        CallName::Assert,
        CallName::Debug,
        CallName::Panic,
        CallName::Fold(
            FunctionName::from_str_unchecked("name"),
            NonZeroPow2Usize::TWO,
        ),
        CallName::ArrayFold(FunctionName::from_str_unchecked("name"), some),
        CallName::ForWhile(FunctionName::from_str_unchecked("name")),
    ];

    functions
        .iter()
        .filter_map(|func| match_callname(func.to_owned()))
        .collect()
}

fn match_callname(call: CallName) -> Option<FunctionTemplate> {
    match call {
        CallName::UnwrapLeft(aliased_type) => {
            let ty = aliased_type.to_string();
            Some(FunctionTemplate::new(
                format!("unwrap_left::<{ty}>"),
                "unwrap_left",
                str_vec![format!("{ty}")],
                str_vec![format!("Either<{ty}, U>")],
                ty,
                "Unwrap left side of `Either`",
            ))
        }
        CallName::UnwrapRight(aliased_type) => {
            let ty = aliased_type.to_string();
            Some(FunctionTemplate::new(
                format!("unwrap_right::<{ty}>"),
                "unwrap_left",
                str_vec![format!("{ty}")],
                str_vec![format!("Either<T, {ty}>")],
                ty,
                "Unwrap right side of `Either`",
            ))
        }
        CallName::Unwrap => Some(FunctionTemplate::simple(
            "unwrap",
            str_vec!["Option<T>"],
            "T",
            "Unwrap `Option` type",
        )),
        CallName::IsNone(aliased_type) => {
            let ty = aliased_type.to_string();
            Some(FunctionTemplate::new(
                format!("is_none::<{ty}>"),
                "is_none",
                str_vec![format!("{ty}")],
                str_vec![format!("Option<{ty}>").as_str()],
                "bool",
                "Check if `Option` is None",
            ))
        }
        CallName::Assert => Some(FunctionTemplate::simple(
            "assert!",
            str_vec!["bool"],
            "",
            "Fails program if argument is 'false'",
        )),
        CallName::Panic => Some(FunctionTemplate::simple(
            "panic!",
            str_vec![],
            "",
            "Fails program",
        )),
        CallName::Debug => Some(FunctionTemplate::simple(
            "dbg!",
            str_vec!["T"],
            "T",
            "Print value and return it",
        )),
        CallName::Fold(_, _) => Some(FunctionTemplate::new(
            "fold::<F, B>",
            "fold",
            str_vec!["F", "B"],
            str_vec!["iter", "init"],
            "B",
            "Fold operation over an iterator",
        )),
        CallName::ArrayFold(_, _) => Some(FunctionTemplate::new(
            "array_fold::<F, N>",
            "array_fold",
            str_vec!["F", "N"],
            str_vec!["array", "init"],
            "B",
            "Fold operation over an array of size N",
        )),
        CallName::ForWhile(_) => Some(FunctionTemplate::new(
            "for_while::<F>",
            "for_while",
            str_vec!["F"],
            str_vec!["condition", "body"],
            "()",
            "While loop with a function",
        )),
        // TODO: implement TypeCast definition
        CallName::Jet(_) | CallName::TypeCast(_) | CallName::Custom(_) => None,
    }
}
