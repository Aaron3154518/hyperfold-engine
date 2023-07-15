use std::str::pattern::Pattern;

// Search from position to position
pub trait FindFrom<'a, P> {
    fn length(&self) -> usize;

    fn find_from(&'a self, p: P, pos: usize) -> Option<usize> {
        self.find_from_to(p, pos, self.length())
    }

    fn find_to(&'a self, p: P, pos: usize) -> Option<usize> {
        self.find_from_to(p, 0, pos)
    }

    fn find_from_to(&'a self, p: P, pos1: usize, pos2: usize) -> Option<usize>;
}

impl<'a, P> FindFrom<'a, P> for String
where
    P: Pattern<'a>,
{
    fn length(&self) -> usize {
        self.len()
    }

    fn find_from_to(&'a self, pat: P, pos1: usize, pos2: usize) -> Option<usize> {
        self[pos1..pos2].find(pat).map(|idx| idx + pos1)
    }
}

impl<'a, P> FindFrom<'a, P> for &str
where
    P: Pattern<'a>,
{
    fn length(&self) -> usize {
        self.len()
    }

    fn find_from_to(&'a self, pat: P, pos1: usize, pos2: usize) -> Option<usize> {
        self[pos1..pos2].find(pat).map(|idx| idx + pos1)
    }
}

impl<'a, F, T> FindFrom<'a, F> for Vec<T>
where
    T: 'a,
    F: Fn(&'a T) -> bool,
{
    fn length(&self) -> usize {
        self.len()
    }

    fn find_from_to(&'a self, f: F, pos1: usize, pos2: usize) -> Option<usize> {
        self[pos1..pos2].iter().position(f).map(|idx| idx + pos1)
    }
}
