/// Wrapper for a [`f64`] that is finite (i.e. not NaN nor infinite).
///
/// # Layout
///
/// `Num` has the same layout as `f64`.
#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(transparent)]
pub struct Num(f64);

/// `Num` can implement `Eq` because NaN is ruled out.
impl Eq for Num {}

impl Num {
    #[must_use]
    #[inline]
    pub fn new(f: f64) -> Option<Self> {
        if f.is_finite() {
            Some(Self(f))
        } else {
            None
        }
    }

    #[inline]
    pub fn get(self) -> f64 {
        self.0
    }
}

macro_rules! num_impl_from {
    ($param:ident: $typ:ty) => {
        impl From<$typ> for Num {
            #[inline]
            fn from($param: $typ) -> Self {
                Self(f64::from($param))
            }
        }
    };
}

num_impl_from!(i: i32);
num_impl_from!(u: u32);
