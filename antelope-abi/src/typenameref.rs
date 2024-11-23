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

    // FIXME: should this be recursive? ie: what is `fundamental_type("int[]?")` ?
    pub fn fundamental_type(&self) -> TypeNameRef<'a> {
        if self.is_array() {
            TypeNameRef(&self.0[..self.0.len() - 2])
        }
        else if self.is_sized_array() {
            TypeNameRef(&self.0[..self.0.rfind('[').unwrap()])  // unwrap is safe here
        }
        else if self.is_optional() {
            TypeNameRef(&self.0[..self.0.len() - 1])
        }
        else {
            *self
        }
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

impl<'a> fmt::Debug for TypeNameRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> fmt::Display for TypeNameRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// impl<'a, T> TryFrom<T> for TypeNameRef<'a> {
//     type Error = ();
//
//     fn try_from(value: T) -> Result<Self, Self::Error> {
//         &str::try_from(value)
//     }
//
// }

// impl<'a> From<TypeNameRef<'a>> for AntelopeType {
//     // type Error = ();

//     fn from(value: TypeNameRef<'a>) -> Self {
//         AntelopeType::from(value)
//     }

// }

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
