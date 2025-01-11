use core::{
    fmt::Debug,
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Add, AddAssign, Deref, Index, IndexMut},
};

use generic_array::{ArrayLength, GenericArray};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct Wrapping<N: ArrayLength>(usize, PhantomData<N>);

impl<N: ArrayLength> Deref for Wrapping<N> {
    type Target = usize;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<N: ArrayLength> Wrapping<N> {
    const ZERO: Self = Self(0, PhantomData);

    #[inline]
    const fn inc(&mut self) {
        if self.0 == N::USIZE - 1 {
            self.0 = 0;
        } else {
            self.0 += 1;
        }
    }

    #[inline]
    const fn dec(&mut self) {
        if let Some(m1) = self.0.checked_sub(1) {
            self.0 = m1;
        } else {
            self.0 = N::USIZE - 1;
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct Bounded<N: ArrayLength>(usize, PhantomData<N>);

impl<N: ArrayLength> PartialEq for Bounded<N> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<N: ArrayLength> Deref for Bounded<N> {
    type Target = usize;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<N: ArrayLength> Bounded<N> {
    const ZERO: Self = Self(0, PhantomData);

    #[inline]
    const fn inc(mut self) -> Result<Self, Self> {
        if self.0 == N::USIZE {
            Err(self)
        } else {
            self.0 += 1;
            Ok(self)
        }
    }

    #[inline]
    const fn dec(self) -> Result<Self, Self> {
        if let Some(m1) = self.0.checked_sub(1) {
            Ok(Self(m1, PhantomData))
        } else {
            Err(self)
        }
    }
}

impl<N: ArrayLength> Add<Bounded<N>> for Wrapping<N> {
    type Output = Self;

    #[inline]
    fn add(mut self, rhs: Bounded<N>) -> Self::Output {
        self += rhs;
        self
    }
}

impl<N: ArrayLength> AddAssign<Bounded<N>> for Wrapping<N> {
    #[inline]
    fn add_assign(&mut self, rhs: Bounded<N>) {
        let (mut sum, ov) = self.0.overflowing_add(rhs.0);
        if ov || sum >= N::USIZE {
            sum = sum.wrapping_sub(N::USIZE);
        }

        debug_assert!(sum < N::USIZE);

        self.0 = sum;
    }
}

pub(crate) struct Dequeue<T, N: ArrayLength> {
    data: GenericArray<MaybeUninit<T>, N>,
    start: Wrapping<N>,
    len: Bounded<N>,
}

impl<T: Debug, N: ArrayLength> Debug for Dequeue<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Dequeue")
            .field("data", &self.data)
            .field("start", &*self.start)
            .field("len", &*self.len)
            .finish()
    }
}

impl<T: Clone, N: ArrayLength> Clone for Dequeue<T, N> {
    fn clone(&self) -> Self {
        let (slice1, slice2) = self.slices();
        let mut data = GenericArray::uninit();

        let mut data_iter = data.iter_mut();
        for (src, dst) in slice1.iter().zip(&mut data_iter) {
            dst.write(src.clone());
        }
        for (src, dst) in slice2.iter().zip(&mut data_iter) {
            dst.write(src.clone());
        }

        Self {
            data,
            start: Wrapping::ZERO,
            len: self.len,
        }
    }
}

#[must_use = "Contains information on whether the push is actually successful"]
pub(crate) enum PushStatus<T> {
    Success,
    Rejected(T),
}

impl<T> PushStatus<T> {
    pub(crate) fn assert(self) {
        match self {
            PushStatus::Success => {}
            PushStatus::Rejected(_) => {
                panic!("Dequeue out of capacity, failed to push the element")
            }
        }
    }
}

// WARN: make use of `const` on mutating method, once `GenericArray` allows it
impl<T, N: ArrayLength> Dequeue<T, N> {
    #[inline]
    pub(crate) const fn new() -> Self {
        Self {
            data: GenericArray::uninit(),
            len: Bounded::ZERO,
            start: Wrapping::ZERO,
        }
    }

    #[inline]
    fn write_at(&mut self, pos: Bounded<N>, item: T) {
        self.data[*(self.start + pos)].write(item);
    }

    pub(crate) fn push_back(&mut self, item: T) -> PushStatus<T> {
        match self.len.inc() {
            Ok(incremented) => {
                // there is more space in the array

                self.write_at(self.len, item);
                self.len = incremented;

                PushStatus::Success
            }
            Err(_unchanged) => {
                // no more space - reject
                PushStatus::Rejected(item)
            }
        }
    }

    pub(crate) fn push_front(&mut self, item: T) -> PushStatus<T> {
        match self.len.inc() {
            Ok(incremented) => {
                // there is more space in the array
                //
                // move starting position and write to 0

                self.start.dec();
                self.write_at(Bounded::ZERO, item);
                self.len = incremented;

                PushStatus::Success
            }
            Err(_unchanged) => {
                // no more space - reject
                PushStatus::Rejected(item)
            }
        }
    }

    #[inline]
    unsafe fn overwrite_at(&mut self, pos: Bounded<N>, item: T) {
        let pos = *(self.start + pos);
        self.data[pos].assume_init_drop();
        self.data[pos].write(item);
    }

    pub(crate) fn push_back_overwrite(&mut self, item: T) {
        match self.len.inc() {
            Ok(incremented) => {
                // there is more space in the array

                self.write_at(self.len, item);
                self.len = incremented;
            }
            Err(_unchanged) => {
                // no more space - overwrite
                //
                // overwrite to 0 and move the start

                // SAFETY: array is full, so all positions should contain valid data
                unsafe {
                    self.overwrite_at(Bounded::ZERO, item);
                }
                self.start.inc();
            }
        }
    }

    pub(crate) fn push_front_overwrite(&mut self, item: T) {
        match self.len.inc() {
            Ok(incremented) => {
                // there is more space in the array
                //
                // move starting position and write to 0

                self.start.dec();
                self.write_at(Bounded::ZERO, item);
                self.len = incremented;
            }
            Err(_unchanged) => {
                // no more space - overwrite
                //
                // move starting position and overwrite to 0

                self.start.dec();
                // SAFETY: array is full, so all positions should contain valid data
                unsafe {
                    self.overwrite_at(Bounded::ZERO, item);
                }
            }
        }
    }

    #[inline]
    unsafe fn take_at(&mut self, pos: Bounded<N>) -> T {
        self.data[*(self.start + pos)].assume_init_read()
    }

    pub(crate) fn pop_back(&mut self) -> Option<T> {
        match self.len.dec() {
            Ok(len_m1) => {
                // take from logical `len-1`
                self.len = len_m1;
                // SAFETY:
                // Logical positions from `0` to `len-1` inclusive always contain valid data
                unsafe { Some(self.take_at(len_m1)) }
            }
            Err(_empty) => None,
        }
    }

    pub(crate) fn pop_front(&mut self) -> Option<T> {
        match self.len.dec() {
            Ok(len_m1) => {
                // take from logical `0`, then move the start
                // SAFETY:
                // Logical positions from `0` to `len-1` inclusive always contain valid data.
                let res = unsafe { Some(self.take_at(Bounded::ZERO)) };
                self.len = len_m1;
                self.start.inc();
                res
            }
            Err(_empty) => None,
        }
    }

    #[inline]
    unsafe fn read_at(&self, pos: Bounded<N>) -> &T {
        self.data[*(self.start + pos)].assume_init_ref()
    }

    pub(crate) fn get(&self, i: usize) -> Option<&T> {
        if i < *self.len {
            // SAFETY:
            // Logical positions from `0` to `len-1` contain valid elements. Above condition checks that index is bounded by `len`.
            //
            // Due to absolute order, it is then bounded to `LEN` too, so it is ok to create `Bounded`.
            unsafe { Some(self.read_at(Bounded(i, PhantomData))) }
        } else {
            None
        }
    }

    #[inline]
    unsafe fn read_at_mut(&mut self, pos: Bounded<N>) -> &mut T {
        self.data[*(self.start + pos)].assume_init_mut()
    }

    pub(crate) fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        if i < *self.len {
            // SAFETY:
            // Logical positions from `0` to `len-1` contain valid elements. Above condition checks that index is bounded by `len`.
            //
            // Due to absolute order, it is then bounded to `LEN` too, so it is ok to create `Bounded`.
            unsafe { Some(self.read_at_mut(Bounded(i, PhantomData))) }
        } else {
            None
        }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        *self.len
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.len == Bounded::ZERO
    }

    fn slices(&self) -> (&[T], &[T]) {
        let Ok(len_m1) = self.len.dec() else {
            // vec is empty
            return (&[], &[]);
        };
        let last_position = self.start + len_m1;

        // SAFETY:
        // Last element is contained on the logical position `len-1`
        //
        // If last position comes physically after the start, all elements are in a single slice
        //
        // If last position comes physically before the start, elements are spread from the start to the physical end, and from physical start to logical end
        unsafe {
            use core::ptr::from_ref;
            if *last_position >= *self.start {
                // has a single slice
                #[cfg(debug_assertions)]
                {
                    for i in *self.start..=*last_position {
                        self.data[i].assume_init_ref();
                    }
                }
                (
                    &*(from_ref(&self.data[*self.start..=*last_position]) as *const [T]),
                    &[],
                )
            } else {
                // has two slices
                #[cfg(debug_assertions)]
                {
                    for i in *self.start..N::USIZE {
                        self.data[i].assume_init_ref();
                    }
                }
                #[cfg(debug_assertions)]
                {
                    for i in 0..=*last_position {
                        self.data[i].assume_init_ref();
                    }
                }
                (
                    &*(from_ref(&self.data[*self.start..]) as *const [T]),
                    &*(from_ref(&self.data[..=*last_position]) as *const [T]),
                )
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        let Ok(len_m1) = self.len.dec() else {
            // vec is empty, nothing to do
            return;
        };
        let last_position = self.start + len_m1;

        // SAFETY:
        // Last element is contained on the logical position `len-1`
        //
        // If last position comes physically after the start, all elements are in a single slice
        //
        // If last position comes physically before the start, elements are spread from the start to the physical end, and from physical start to logical end
        unsafe {
            if *last_position >= *self.start {
                // has a single slice
                for i in *self.start..=*last_position {
                    self.data[i].assume_init_drop();
                }
            } else {
                // has two slices

                for i in *self.start..N::USIZE {
                    self.data[i].assume_init_drop();
                }
                for i in 0..=*last_position {
                    self.data[i].assume_init_drop();
                }
            }
        }

        self.start = Wrapping::ZERO;
        self.len = Bounded::ZERO;
    }
}

impl<T, N: ArrayLength> Default for Dequeue<T, N> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, N: ArrayLength> Index<usize> for Dequeue<T, N> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        if let Some(res) = self.get(index) {
            res
        } else {
            panic!(
                "Index out of bounds: index {index}, but length {}",
                *self.len
            );
        }
    }
}

impl<T, N: ArrayLength> IndexMut<usize> for Dequeue<T, N> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let len = self.len;
        if let Some(res) = self.get_mut(index) {
            res
        } else {
            panic!("Index out of bounds: index {index}, but length {}", *len);
        }
    }
}

impl<T, N: ArrayLength> Drop for Dequeue<T, N> {
    #[inline]
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(any(kani, test))]
mod tests;
