use crate::completion::types::FunctionCompletionTemplate;

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

/// Get completion of buildin functions. They are all defined in `simplicityhl::parse::CallName`
pub fn get_builtin_functions() -> Vec<FunctionCompletionTemplate> {
    vec![
        FunctionCompletionTemplate::simple(
            "assert!",
            str_vec!["bool"],
            "",
            "Fails program if argument is 'false'",
        ),
        FunctionCompletionTemplate::simple(
            "dbg!",
            str_vec!["type"],
            "type",
            "Print value and return it",
        ),
        FunctionCompletionTemplate::simple("panic!", str_vec![], "", "Fails program"),
        FunctionCompletionTemplate::new(
            "unwrap_left::<T>",
            "unwrap_left",
            str_vec!["T"],
            str_vec!["Either<T, U>"],
            "T",
            "Unwrap left side of Either",
        ),
        FunctionCompletionTemplate::new(
            "unwrap_right::<U>",
            "unwrap_right",
            str_vec!["U"],
            str_vec!["Either<T, U>"],
            "U",
            "Unwrap right side of Either",
        ),
        FunctionCompletionTemplate::new(
            "is_none::<T>",
            "is_none",
            str_vec!["T"],
            str_vec!["Option<T>"],
            "bool",
            "Check if Option is None",
        ),
        FunctionCompletionTemplate::new(
            "fold::<F, B>",
            "fold",
            str_vec!["F", "B"],
            str_vec!["iter", "init"],
            "B",
            "Fold operation over an iterator",
        ),
        FunctionCompletionTemplate::new(
            "array_fold::<F, N>",
            "array_fold",
            str_vec!["F", "N"],
            str_vec!["array", "init"],
            "B",
            "Fold operation over an array of size N",
        ),
        FunctionCompletionTemplate::new(
            "for_while::<F>",
            "for_while",
            str_vec!["F"],
            str_vec!["condition", "body"],
            "()",
            "While loop with a function",
        ),
    ]
}
