
/// Define the `From` conversion from an error type to another, where the first
/// is the source of the second. It will use the given `Snafu` context selector
/// to do so.
#[macro_export]
macro_rules! impl_auto_error_conversion {
    ($src:ty, $target:ty, $snafu:ident) => {
        impl From<$src> for $target {
            fn from(value: $src) -> $target {
                snafu::IntoError::into_error($snafu, value)
            }
        }
    };
}
