
// thanks https://stackoverflow.com/questions/74841526/why-does-stditerpeekablepeek-mutably-borrow-the-self-argument

#![allow(unused)]

pub struct CachedLookahead<I: Iterator, const SIZE: usize> {
    iter: I,
    next_items: [Option<I::Item>; SIZE],
}

impl<I: Iterator, const SIZE: usize> CachedLookahead<I, SIZE> {
    pub fn new(mut iter: I) -> Self {
        let mut next_items: [Option<I::Item>; SIZE] = [(); SIZE].map(|_| None);
        for i in 0..SIZE {
            next_items[i] = iter.next();
        }
        Self { iter, next_items }
    }

    pub fn lookahead(&self) -> &[Option<I::Item>; SIZE] {
        &self.next_items
    }

    pub fn peek(&self) -> Option<&I::Item> {
        self.next_items[0].as_ref()
    }
}

impl<I: Iterator, const SIZE: usize> Iterator for CachedLookahead<I, SIZE> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_items.rotate_left(1);
        std::mem::replace(&mut self.next_items[SIZE - 1], self.iter.next())
    }
}

pub trait CreateCachedLookahead: Iterator + Sized {
    fn lookahead_cached<const SIZE: usize>(self) -> CachedLookahead<Self, SIZE>;
    fn peekable_cached(self) -> CachedLookahead<Self, 1>;
}

impl CreateCachedLookahead for pulldown_cmark::Parser<'_, '_> {
    fn lookahead_cached<const SIZE: usize>(self) -> CachedLookahead<Self, SIZE> {
        CachedLookahead::new(self)
    }

    fn peekable_cached(self) -> CachedLookahead<Self, 1> {
        CachedLookahead::new(self)
    }
}
