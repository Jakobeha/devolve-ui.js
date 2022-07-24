/// A type with some default arguments but some required
pub trait PartialDefault {
    type RequiredArgs;

    fn partial_default(required_args: Self::RequiredArgs) -> Self;
}

impl <T: Default> PartialDefault for T {
    type RequiredArgs = ();

    fn partial_default((): Self::RequiredArgs) -> Self {
        Self::default()
    }
}
