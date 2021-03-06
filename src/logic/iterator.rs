use std::iter::Zip;

/// This first iterator wraps the standard library `Zip` iterator and flattens nested tuples of values returned to a 
/// flat list.
macro_rules! impl_zip {
    ($name: ident, $zip_type: ty, $m_stuff: expr, $($T: ident),*) => {
        pub struct $name<A: Iterator, $($T: Iterator,)*> {
            inner: $zip_type,
        }

        impl<A: Iterator, $($T: Iterator,)*> $name<A, $($T,)*> {
            #[allow(non_snake_case)]
            pub fn new (A: A, $($T: $T,)*) -> Self {
                Self {
                    inner: A$(.zip($T))*
                }
            }
        }

        impl<A: Iterator, $($T: Iterator,)*> Iterator for $name<A, $($T,)*> {
            type Item = (A::Item, $($T::Item,)*);

            #[inline(always)]
            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next().map($m_stuff)
            }
            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }
        }
    };
}

// TODO
// HOW DO YOU WRITE RECURSIVE MACROS
// FUCK
impl_zip! {Zip3, Zip<Zip<A, B>, C>, |((a, b), c)| {(a, b, c)}, B, C}
impl_zip! {Zip4, Zip<Zip<Zip<A, B>, C>, D>, |(((a, b), c), d)| {(a, b, c, d)}, B, C, D}
impl_zip! {Zip5, Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, |((((a, b), c), d), e)| {(a, b, c, d, e)}, B, C, D, E}
impl_zip! {Zip6, Zip<Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, F>, |(((((a, b), c), d), e), f)| {(a, b, c, d, e, f)}, B, C, D, E, F}
impl_zip! {Zip7, Zip<Zip<Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, F>, G>, |((((((a, b), c), d), e), f), g)| {(a, b, c, d, e, f, g)}, B, C, D, E, F, G}
impl_zip! {Zip8, Zip<Zip<Zip<Zip<Zip<Zip<Zip<A, B>, C>, D>, E>, F>, G>, H>, |(((((((a, b), c), d), e), f), g), h)| {(a, b, c, d, e, f, g, h)}, B, C, D, E, F, G, H}

/// A series of iterators of the same type that are traversed in a row.
pub struct ChainedIterator<I: Iterator> {
    current_iter: Option<I>,
    iterators: Vec<I>,
}

impl<I: Iterator> ChainedIterator<I> {
    pub fn new(mut iterators: Vec<I>) -> Self {
        let current_iter = iterators.pop();
        Self {
            current_iter,
            iterators,
        }
    }
}

impl<I: Iterator> Iterator for ChainedIterator<I> {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Chain the iterators together.
        // If the end of one iterator is reached go to the next.
        match self.current_iter {
            Some(ref mut iter) => match iter.next() {
                None => {
                    self.current_iter = self.iterators.pop();
                    if let Some(ref mut iter) = self.current_iter {
                        iter.next()
                    } else {
                        None
                    }
                }
                item => item,
            },
            None => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let mut min = 0;
        let mut max = 0;

        if let Some(current_iter) = &self.current_iter {
            let (i_min, i_max) = current_iter.size_hint();
            min += i_min;
            max += i_max.unwrap();
        }

        for i in self.iterators.iter() {
            let (i_min, i_max) = i.size_hint();
            min += i_min;
            // This function is designed under the assumption that all
            // iterators passed in implement size_hint.
            max += i_max.unwrap();
        }
        (min, Some(max))
    }
}
