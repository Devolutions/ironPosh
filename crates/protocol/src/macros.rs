#[macro_export]
macro_rules! opt_header {
    ($vec:ident, $($field:expr),* $(,)?) => {
        $(
            if let Some(h) = $field {
                $vec.push(h.into_element());
            }
        )*
    };
}
