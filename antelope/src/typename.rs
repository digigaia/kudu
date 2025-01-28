use std::fmt;

use crate::AntelopeType;

/// Newtype wrapper for a `&str` representing a type name that adds a few
/// convenience methods.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeName<'a>(pub &'a str);

impl<'a> TypeName<'a> {
    pub fn is_array(&self) -> bool {
        self.0.ends_with("[]")
    }

    pub fn is_sized_array(&self) -> bool {
        match (self.0.rfind('['), self.0.rfind(']')) {
            (Some(pos1), Some(pos2)) => {
                if pos1 + 1 < pos2 {
                    self.0[pos1 + 1..pos2].chars().all(|c| c.is_ascii_digit())
                }
                else {
                    false
                }
            },
            _ => false,
        }
    }

    pub fn is_optional(&self) -> bool {
        self.0.ends_with('?')
    }

    pub fn is_integer(&self) -> bool {
        self.0.starts_with("int") || self.0.starts_with("uint")
    }

    /// Return the fundamental type for the given type, ie: the type with a
    /// special designator (?/optional, []/array) removed.
    ///
    /// Note that this doesn't work recursively and only work by removing the last
    /// suffix, if you want the base type you have to call this method recursively
    /// yourself.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use antelope::TypeName;
    /// assert_eq!(TypeName("int8"), TypeName("int8"));
    /// assert_eq!(TypeName("int8[]"), TypeName("int8"));
    /// assert_eq!(TypeName("int8[][]"), TypeName("int8[]"));
    /// assert_eq!(TypeName("int8[][]?"), TypeName("int8[][]"));
    /// ```
    pub fn fundamental_type(&self) -> TypeName<'a> {
        if self.is_array() {
            TypeName(&self.0[..self.0.len() - 2])
        }
        else if self.is_sized_array() {
            TypeName(&self.0[..self.0.rfind('[').unwrap()])  // safe unwrap
        }
        else if self.is_optional() {
            TypeName(&self.0[..self.0.len() - 1])
        }
        else {
            *self
        }
    }

    pub fn has_bin_extension(&self) -> bool {
        self.0.ends_with('$')
    }

    pub fn remove_bin_extension(&self) -> TypeName<'a> {
        if self.0.ends_with('$') {
            TypeName(&self.0[..self.0.len()-1])
        }
        else {
            *self
        }
    }
}

impl fmt::Debug for TypeName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for TypeName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> From<&'a String> for TypeName<'a> {
    fn from(t: &String) -> TypeName {
        TypeName(t.as_str())
    }
}

impl<'a> From<&'a str> for TypeName<'a> {
    fn from(t: &str) -> TypeName {
        TypeName(t)
    }
}

impl<'a> From<TypeName<'a>> for &'a str {
    fn from(t: TypeName) -> &str {
        t.0
    }
}

impl<'a> TryFrom<TypeName<'a>> for AntelopeType {
    type Error = strum::ParseError;

    fn try_from(value: TypeName<'a>) -> Result<Self, Self::Error> {
        AntelopeType::try_from(value.0)
    }

}
