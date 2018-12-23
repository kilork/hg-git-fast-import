use std::iter::Peekable;
use std::slice::Iter;

pub fn conditional_multi_iter<'a, T, I, A, P>(
    sources: &'a [Vec<T>],
    aggregate: A,
    predicate: P,
) -> ConditionalMultiIterator<'a, T, I, A, P>
where
    A: Fn(&[Option<&&T>]) -> I,
    P: Fn(&Option<&&T>, &I) -> bool,
{
    ConditionalMultiIterator {
        sources: sources.iter().map(|x| x.iter().peekable()).collect(),
        aggregate,
        predicate,
    }
}

pub struct ConditionalMultiIterator<
    'a,
    T,
    I,
    A: Fn(&[Option<&&T>]) -> I,
    P: Fn(&Option<&&T>, &I) -> bool,
> {
    sources: Vec<Peekable<Iter<'a, T>>>,
    aggregate: A,
    predicate: P,
}

impl<'a, T: 'a, I, A, P> Iterator for ConditionalMultiIterator<'a, T, I, A, P>
where
    A: Fn(&[Option<&&T>]) -> I,
    P: Fn(&Option<&&T>, &I) -> bool,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let input: Vec<_> = self.sources.iter_mut().map(|x| x.peek()).collect();
        let aggregated = (self.aggregate)(&input);
        for source in &mut self.sources {
            let input = source.peek();
            if (self.predicate)(&input, &aggregated) {
                return source.next();
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn conditional_multi_iter_check() {
        let data = vec![vec![(2, 1), (1, 2), (3, 3)], vec![(1, 4), (2, 5)]];
        let mut iter = super::conditional_multi_iter(
            &data,
            |x| x.iter().filter_map(|x| x.map(|x| x.0)).min(),
            |x, y| y == &x.map(|x| x.0),
        );
        assert_eq!(iter.next(), Some(&(1, 4)));
        assert_eq!(iter.next(), Some(&(2, 1)));
        assert_eq!(iter.next(), Some(&(1, 2)));
        assert_eq!(iter.next(), Some(&(2, 5)));
        assert_eq!(iter.next(), Some(&(3, 3)));
        assert_eq!(iter.next(), None);
    }
}
