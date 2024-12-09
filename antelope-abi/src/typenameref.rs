use std::fmt;
use antelope_core::AntelopeType;

// TODO: derive more? e.g. PartialEq, Eq, Hash, etc.

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TypeNameRef<'a>(pub &'a str);

impl<'a> TypeNameRef<'a> {
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
    /// assert_eq!(TypeNameRef("int8"), TypeNameRef("int8"));
    /// assert_eq!(TypeNameRef("int8[]"), TypeNameRef("int8"));
    /// assert_eq!(TypeNameRef("int8[][]"), TypeNameRef("int8[]"));
    /// assert_eq!(TypeNameRef("int8[][]?"), TypeNameRef("int8[][]"));
    /// ```
    pub fn fundamental_type(&self) -> TypeNameRef<'a> {
        if self.is_array() {
            TypeNameRef(&self.0[..self.0.len() - 2])
        }
        else if self.is_sized_array() {
            TypeNameRef(&self.0[..self.0.rfind('[').unwrap()])  // safe unwrap
        }
        else if self.is_optional() {
            TypeNameRef(&self.0[..self.0.len() - 1])
        }
        else {
            *self
        }
    }

    pub fn has_bin_extension(&self) -> bool {
        self.0.ends_with('$')
    }

    pub fn remove_bin_extension(&self) -> TypeNameRef<'a> {
        if self.0.ends_with('$') {
            TypeNameRef(&self.0[..self.0.len()-1])
        }
        else {
            *self
        }
    }
}

impl fmt::Debug for TypeNameRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for TypeNameRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> From<&'a String> for TypeNameRef<'a> {
    fn from(t: &String) -> TypeNameRef {
        TypeNameRef(t.as_str())
    }
}

impl<'a> From<&'a str> for TypeNameRef<'a> {
    fn from(t: &str) -> TypeNameRef {
        TypeNameRef(t)
    }
}

impl<'a> From<TypeNameRef<'a>> for &'a str {
    fn from(t: TypeNameRef) -> &str {
        t.0
    }
}

impl<'a> TryFrom<TypeNameRef<'a>> for AntelopeType {
    type Error = strum::ParseError;

    fn try_from(value: TypeNameRef<'a>) -> Result<Self, Self::Error> {
        AntelopeType::try_from(value.0)
    }

}
