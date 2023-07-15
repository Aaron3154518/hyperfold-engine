pub trait ExpandEnum<const N: usize>
where
    Self: Sized,
{
    const LEN: usize = N;
    const VARIANTS: [Self; N];
}
