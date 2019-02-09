use std::iter::Peekable;

pub fn conditional_multi_iter<'a, X, I, A, P>(
    sources: Vec<X>,
    aggregate: A,
    predicate: P,
) -> ConditionalMultiIterator<X, A, I, P>
where
    A: Fn(&[Option<&X::Item>]) -> I,
    P: Fn(&Option<&X::Item>, &I) -> bool,
    X: IntoIterator,
{
    ConditionalMultiIterator {
        sources: sources
            .into_iter()
            .map(|x| x.into_iter().peekable())
            .collect(),
        aggregate,
        predicate,
    }
}

pub struct ConditionalMultiIterator<X: IntoIterator, A, I, P>
where
    A: Fn(&[Option<&X::Item>]) -> I,
    P: Fn(&Option<&X::Item>, &I) -> bool,
{
    sources: Vec<Peekable<X::IntoIter>>,
    aggregate: A,
    predicate: P,
}

impl<X, A, I, P> Iterator for ConditionalMultiIterator<X, A, I, P>
where
    X: IntoIterator,
    A: Fn(&[Option<&X::Item>]) -> I,
    P: Fn(&Option<&X::Item>, &I) -> bool,
{
    type Item = X::Item;

    fn next(&mut self) -> std::option::Option<Self::Item> {
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
            data,
            |x| x.iter().filter_map(|x| x.map(|x| x.0)).min(),
            |x, y| y == &x.map(|x| x.0),
        );
        assert_eq!(iter.next(), Some((1, 4)));
        assert_eq!(iter.next(), Some((2, 1)));
        assert_eq!(iter.next(), Some((1, 2)));
        assert_eq!(iter.next(), Some((2, 5)));
        assert_eq!(iter.next(), Some((3, 3)));
        assert_eq!(iter.next(), None);
    }
}
