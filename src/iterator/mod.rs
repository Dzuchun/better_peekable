use core::{
    fmt::Debug,
    iter::FusedIterator,
    marker::PhantomData,
    ops::{Add, Deref, Sub},
};

use generic_array::{
    typenum::{self, Const},
    ArrayLength, GenericArray, IntoArrayLength,
};

use crate::dequeue::Dequeue;

type U1 = typenum::U1;
type U2 = typenum::U2;
type U3 = typenum::U3;

pub struct BPeekN<I: Iterator, N: ArrayLength> {
    inner: I,
    queue: Dequeue<I::Item, N>,
}

impl<I: Iterator, N: ArrayLength> Debug for BPeekN<I, N>
where
    I: Debug,
    I::Item: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BPeekN")
            .field("inner", &self.inner)
            .field("queue", &self.queue)
            .field("LEN", &N::USIZE)
            .finish()
    }
}

impl<I: Iterator, N: ArrayLength> Clone for BPeekN<I, N>
where
    I: Clone,
    I::Item: Clone,
{
    fn clone(&self) -> Self {
        BPeekN {
            inner: self.inner.clone(),
            queue: self.queue.clone(),
        }
    }
}

impl<I: Iterator, N: ArrayLength> Iterator for BPeekN<I, N> {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.queue.pop_front() {
            Some(buffered) => Some(buffered),
            None => self.inner.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let buffered = self.queue.len();
        let (rest_min, res_max) = self.inner.size_hint();
        (buffered + rest_min, res_max.map(|v| buffered + v))
    }

    #[inline]
    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.queue.len() + self.inner.count()
    }

    #[inline]
    fn last(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        if let Some(inner_last) = self.inner.last() {
            return Some(inner_last);
        }

        self.queue.pop_back()
    }

    #[inline]
    fn nth(&mut self, mut n: usize) -> Option<Self::Item> {
        if n > self.queue.len() {
            n -= self.queue.len();
            self.queue.clear();
            self.inner.nth(n)
        } else {
            for _ in 1..n {
                let _ = self
                    .queue
                    .pop_front()
                    .expect("Must be present, since index of target is less than number of elements in the queue");
            }
            Some(self.queue.pop_front().expect("Must be present, since index of target is less than number of elements in the queue"))
        }
    }

    #[inline]
    fn for_each<F>(mut self, mut f: F)
    where
        Self: Sized,
        F: FnMut(Self::Item),
    {
        while let Some(item) = self.queue.pop_front() {
            f(item);
        }
        self.inner.for_each(f);
    }

    #[inline]
    fn collect<B: FromIterator<Self::Item>>(mut self) -> B
    where
        Self: Sized,
    {
        core::iter::from_fn(|| self.queue.pop_front())
            .chain(self.inner)
            .collect()
    }

    fn partition<B, F>(mut self, mut f: F) -> (B, B)
    where
        Self: Sized,
        B: Default + Extend<Self::Item>,
        F: FnMut(&Self::Item) -> bool,
    {
        let mut true_collection = B::default();
        let mut false_collection = B::default();

        for _ in 0..N::USIZE - self.queue.len() {
            let Some(item) = self.inner.next() else {
                break;
            };

            self.queue.push_back(item).assert();
        }

        let mut next_result = Option::<bool>::None;
        while !self.queue.is_empty() {
            while let Some(first) = self.queue.pop_front() {
                let result = next_result.take().unwrap_or_else(|| f(&first));
                let mut additional = 0;
                while self.queue.get(additional).is_some_and(|val| {
                    if f(val) == result {
                        true
                    } else {
                        next_result = Some(!result);
                        false
                    }
                }) {
                    additional += 1;
                }
                let collection = if result {
                    &mut true_collection
                } else {
                    &mut false_collection
                };

                collection.extend(core::iter::once(first).chain((0..additional).map(|_| {
                    self.queue
                        .pop_front()
                        .expect("Contains at least as many elements as passed the test")
                })));
            }

            for _ in 0..N::USIZE {
                let Some(item) = self.inner.next() else {
                    break;
                };

                self.queue.push_back(item).assert();
            }
        }

        (true_collection, false_collection)
    }

    #[inline]
    fn fold<B, F>(mut self, mut init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        while let Some(item) = self.queue.pop_front() {
            init = f(init, item);
        }
        self.inner.fold(init, f)
    }

    #[inline]
    fn reduce<F>(mut self, mut f: F) -> Option<Self::Item>
    where
        Self: Sized,
        F: FnMut(Self::Item, Self::Item) -> Self::Item,
    {
        if let Some(mut res) = self.queue.pop_front() {
            while let Some(item) = self.queue.pop_front() {
                res = f(res, item);
            }
            Some(self.inner.fold(res, f))
        } else {
            self.inner.reduce(f)
        }
    }

    #[inline]
    fn all<F>(&mut self, mut f: F) -> bool
    where
        Self: Sized,
        F: FnMut(Self::Item) -> bool,
    {
        while let Some(item) = self.queue.pop_front() {
            if !f(item) {
                return false;
            };
        }
        self.inner.all(f)
    }

    #[inline]
    fn any<F>(&mut self, mut f: F) -> bool
    where
        Self: Sized,
        F: FnMut(Self::Item) -> bool,
    {
        while let Some(item) = self.queue.pop_front() {
            if f(item) {
                return true;
            };
        }
        for item in self.inner.by_ref() {
            if f(item) {
                return true;
            };
        }
        self.inner.any(f)
    }

    #[inline]
    fn find<P>(&mut self, mut predicate: P) -> Option<Self::Item>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        while let Some(item) = self.queue.pop_front() {
            if predicate(&item) {
                return Some(item);
            };
        }

        self.inner.find(predicate)
    }

    #[inline]
    fn find_map<B, F>(&mut self, mut f: F) -> Option<B>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Option<B>,
    {
        while let Some(item) = self.queue.pop_front() {
            if let Some(res) = f(item) {
                return Some(res);
            };
        }

        self.inner.find_map(f)
    }

    #[inline]
    fn position<P>(&mut self, mut predicate: P) -> Option<usize>
    where
        Self: Sized,
        P: FnMut(Self::Item) -> bool,
    {
        let mut skipped = 0;
        while let Some(item) = self.queue.pop_front() {
            if predicate(item) {
                return Some(skipped);
            }
            skipped += 1;
        }
        self.inner
            .position(predicate)
            .map(|pos_inner| skipped + pos_inner)
    }

    // TODO: probably add rest of the methods
}

impl<I: Iterator + DoubleEndedIterator, N: ArrayLength> DoubleEndedIterator for BPeekN<I, N> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        // try inner iterator
        if let Some(inner_item) = self.inner.next_back() {
            return Some(inner_item);
        }

        // try getting from buffer
        self.queue.pop_back()
    }
}

impl<I: Iterator + FusedIterator, N: ArrayLength> FusedIterator for BPeekN<I, N> {}

impl<I: Iterator + ExactSizeIterator, N: ArrayLength> ExactSizeIterator for BPeekN<I, N> {}

impl<I: Iterator, N: ArrayLength> BPeekN<I, N> {
    fn ensure_elements<C: ArrayLength>(&mut self) -> Option<GenericArray<&I::Item, C>>
    where
        N: Sub<C>,
    {
        if self.queue.len() < C::USIZE {
            for _ in 0..C::USIZE - self.queue.len() {
                self.queue.push_back(self.inner.next()?).assert();
                // ^^ always able to push, since number of elements to ensure is statically proven to not be larger than number of elements buffer can hold
            }
        }

        Some(
            (0..C::USIZE)
                .map(|i| {
                    self.queue.get(i).expect(
                        "Rest of the function proves that this element exists in the buffer",
                    )
                })
                .collect(),
        )
    }

    #[inline]
    pub fn bpeek<Off: ArrayLength + Sub<U1>>(&mut self) -> Option<PeekCursor<'_, I, N, Off>>
    where
        N: Sub<Off>,
    {
        let _ = self.ensure_elements::<Off>()?;
        Some(PeekCursor {
            iter: self,
            _phantom: PhantomData,
        })
    }

    #[inline]
    pub fn bpeek1(&mut self) -> Option<PeekCursor<'_, I, N, U1>>
    where
        N: Sub<U1>,
    {
        self.bpeek()
    }

    #[inline]
    pub fn bpeek2(&mut self) -> Option<PeekCursor<'_, I, N, U2>>
    where
        N: Sub<U2>,
    {
        self.bpeek()
    }

    #[inline]
    pub fn bpeek3(&mut self) -> Option<PeekCursor<'_, I, N, U3>>
    where
        N: Sub<U3>,
    {
        self.bpeek()
    }
}

pub struct PeekCursor<'iter, I: Iterator, N: ArrayLength + Sub<Ind>, Ind: ArrayLength + Sub<U1>> {
    iter: &'iter mut BPeekN<I, N>,
    _phantom: PhantomData<Ind>,
}

impl<I: Iterator, N: ArrayLength + Sub<Ind>, Ind: ArrayLength + Sub<U1>> Debug
    for PeekCursor<'_, I, N, Ind>
where
    I: Debug,
    I::Item: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PeekCursor")
            .field("iter", &*self.iter)
            .finish()
    }
}

impl<I: Iterator, N: ArrayLength, Ind: ArrayLength + Sub<U1>> Deref for PeekCursor<'_, I, N, Ind>
where
    N: Sub<Ind>,
{
    type Target = I::Item;

    fn deref(&self) -> &Self::Target {
        self.iter.queue.get(Ind::USIZE - 1).expect(
            "Should be present, since number of buffered elements is ensured on construction",
        )
    }
}

impl<I: Iterator, N: ArrayLength + Sub<Ind>, Ind: ArrayLength + Sub<U1>> PeekCursor<'_, I, N, Ind> {
    ///
    pub fn take_all<const OFF: usize>(self) -> [I::Item; OFF]
    where
        Const<OFF>: IntoArrayLength<ArrayLength = Ind>,
    {
        let array: GenericArray<I::Item, Ind> = (0..Ind::USIZE)
            .map(|_| {
                self.iter
                    .queue
                    .pop_front()
                    .expect("Must be present, number of available elements is ensured statically")
            })
            .collect();
        array.into_array()
    }
}

impl<'iter, I: Iterator, N: ArrayLength + Sub<Ind>, Ind: ArrayLength + Sub<U1>>
    PeekCursor<'iter, I, N, Ind>
{
    ///
    pub fn peek_all<const OFF: usize>(&self) -> [&I::Item; OFF]
    where
        Const<OFF>: IntoArrayLength<ArrayLength = Ind>,
    {
        let array: GenericArray<&I::Item, Ind> = (0..Ind::USIZE)
            .map(|i| {
                self.iter
                    .queue
                    .get(i)
                    .expect("Must be present, number of available elements is ensured statically")
            })
            .collect();
        array.into_array()
    }

    pub fn peek_prev(self) -> PeekCursor<'iter, I, N, <Ind as Sub<U1>>::Output>
    where
        <Ind as Sub<U1>>::Output: ArrayLength + Sub<U1>,
        N: Sub<<Ind as Sub<U1>>::Output>,
    {
        // no checks necessary, all previous elements are available
        PeekCursor {
            iter: self.iter,
            _phantom: PhantomData,
        }
    }

    pub fn peek_forward(self) -> Result<PeekCursor<'iter, I, N, <Ind as Add<U1>>::Output>, Self>
    where
        Ind: Add<U1>,
        <Ind as Add<U1>>::Output: ArrayLength + Sub<U1>,
        N: Sub<<Ind as Add<U1>>::Output>,
    {
        if self.iter.queue.len() <= Ind::USIZE {
            debug_assert_eq!(
                self.iter.queue.len(),
                Ind::USIZE,
                "At this point, number of buffered elements can only be 1 less"
            );
            let Some(last_item) = self.iter.inner.next() else {
                return Err(self);
            };
            self.iter.queue.push_back(last_item).assert();
            // ^^^ must be able to push, buffer capacity is ensured statically
        }
        debug_assert_eq!(
            self.iter.queue.len(),
            Ind::USIZE + 1,
            "At this point, buffer should contain enough elements"
        );
        Ok(PeekCursor {
            iter: self.iter,
            _phantom: PhantomData,
        })
    }
}

pub trait BPeekExt: Iterator + Sized {
    #[inline]
    fn bpeekable<N: ArrayLength>(self) -> BPeekN<Self, N> {
        BPeekN {
            inner: self,
            queue: Dequeue::new(),
        }
    }

    #[inline]
    fn bpeekable1(self) -> BPeekN<Self, U1> {
        self.bpeekable()
    }

    #[inline]
    fn bpeekable2(self) -> BPeekN<Self, U2> {
        self.bpeekable()
    }

    #[inline]
    fn bpeekable3(self) -> BPeekN<Self, U3> {
        self.bpeekable()
    }
}

impl<I: Iterator> BPeekExt for I {}

#[cfg(test)]
mod tests;
