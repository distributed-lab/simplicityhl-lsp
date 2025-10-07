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

#[allow(warnings)]
pub fn match_callname(call: CallName) -> Option<FunctionTemplate> {
    match call {
        CallName::UnwrapLeft(aliased_type) => {
            let ty = aliased_type.to_string();
            Some(FunctionTemplate::new(
                format!("unwrap_left::<{ty}>"),
                "unwrap_left",
                str_vec![format!("{ty}")],
                str_vec![format!("Either<{ty}, U>")],
                ty,
                "Extracts the left variant of an `Either` value.

Returns the left-side value if it exists, otherwise panics.

```simplicityhl
let x: Either<u8, u8> = Left(42);
let y: u8 = unwrap_left::<u8>(x); // 42
```",
            ))
        }
        CallName::UnwrapRight(aliased_type) => {
            let ty = aliased_type.to_string();
            Some(FunctionTemplate::new(
                format!("unwrap_right::<{ty}>"),
                "unwrap_right",
                str_vec![format!("{ty}")],
                str_vec![format!("Either<T, {ty}>")],
                ty,
                "Extracts the right variant of an `Either` value.

Returns the right-side value if it exists, otherwise panics.

```simplicityhl
let x: Either<u8, u8> = Right(128);
let y: u8 = unwrap_right::<u8>(x); // 128
```",
            ))
        }
        CallName::Unwrap => Some(FunctionTemplate::simple(
            "unwrap",
            str_vec!["Option<T>"],
            "T",
            "Unwraps an `Option` value, panicking if it is `None`.

```simplicityhl
let x: Option<u8> = Some(5);
let y: u8 = unwrap(x); // 5
```",
        )),
        CallName::IsNone(aliased_type) => {
            let ty = aliased_type.to_string();
            Some(FunctionTemplate::new(
                format!("is_none::<{ty}>"),
                "is_none",
                str_vec![format!("{ty}")],
                str_vec![format!("Option<{ty}>").as_str()],
                "bool",
                "Checks if an `Option` is `None`.

Returns `true` if the value is `None`, otherwise `false`.
",
            ))
        }
        CallName::Assert => Some(FunctionTemplate::simple(
            "assert!",
            str_vec!["condition: bool"],
            "()",
            "Panics when `condition` is false.",
        )),
        CallName::Panic => Some(FunctionTemplate::simple(
            "panic!",
            str_vec![],
            "()",
            "Unconditionally terminates program execution.",
        )),
        CallName::Debug => Some(FunctionTemplate::simple(
            "dbg!",
            str_vec!["T"],
            "T",
            "Prints a value if debugging symbols is enabled 
            and returns it unchanged.

```simplicityhl
let x: u32 = dbg!(42); // prints 42, returns 42
```",
        )),
        CallName::Fold(_, _) => Some(FunctionTemplate::new(
            "fold::<f, N>",
            "fold",
            str_vec!["f", "N"],
            str_vec!["list: List<E,N>", "initial_accumulator: A"],
            "A",
            "
Fold a list of bounded length by repeatedly applying a function.

- Signature: `fold::<f, N>(list: List<E, N>, initial_accumulator: A) -> A`
- Fold step: `fn f(element: E, acc: A) -> A`
- Note: `N` is a power of two; lists hold fewer than `N` elements.

Example: sum a list of 32-bit integers.

```simplicityhl
fn sum(elt: u32, acc: u32) -> u32 {
    let (_, acc): (bool, u32) = jet::add_32(elt, acc);
    acc
}

fn main() {
    let xs: List<u32, 8> = list![1, 2, 3];
    let s: u32 = fold::<sum, 8>(xs, 0);
    assert!(jet::eq_32(s, 6));
}
```
",
        )),
        CallName::ArrayFold(_, _) => Some(FunctionTemplate::new(
            "array_fold::<f, N>",
            "array_fold",
            str_vec!["f", "N"],
            str_vec!["array: [E; N]", "initial_accumulator: A"],
            "A",
            "
Fold a fixed-size array by repeatedly applying a function.

- Signature: `array_fold::<f, N>(array: [E; N], initial_accumulator: A) -> A`
- Fold step: `fn f(element: E, acc: A) -> A`

Example: sum an array of 7 elements.

```simplicityhl
fn sum(elt: u32, acc: u32) -> u32 {
    let (_, acc): (bool, u32) = jet::add_32(elt, acc);
    acc
}

fn main() {
    let arr: [u32; 7] = [1, 2, 3, 4, 5, 6, 7];
    let sum: u32 = array_fold::<sum, 7>(arr, 0);
    assert!(jet::eq_32(sum, 28));
}
```",
        )),
        CallName::ForWhile(_) => Some(FunctionTemplate::new(
            "for_while::<f>",
            "for_while",
            str_vec!["f"],
            str_vec!["accumulator: A", "context: C"],
            "Either<B, A>",
            "
Run a function `f` repeatedly with a bounded counter. The loop stops early when the function returns a successful value.

- Signature: `for_while::<f>(initial_accumulator: A, readonly_context: C) -> Either<B, A>`
- Loop body: `fn f(acc: A, ctx: C, counter: uN) -> Either<B, A>` where `N ∈ {1, 2, 4, 8, 16}`

Example: stop when `counter == 10`.

```simplicityhl
fn stop_at_10(acc: (), _: (), i: u8) -> Either<u8, ()> {
    match jet::eq_8(i, 10) {
        true => Left(i),   // success → exit loop
        false => Right(acc), // continue with same accumulator
    }
}

fn main() {
    let out: Either<u8, ()> = for_while::<stop_at_10>((), ());
    assert!(jet::eq_8(10, unwrap_left::<()>(out)));
}
```
",
        )),
        // TODO: implement TypeCast definition
        CallName::Jet(_) | CallName::TypeCast(_) | CallName::Custom(_) => None,
    }
}
