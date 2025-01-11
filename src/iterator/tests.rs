use core::hash::Hasher;

use super::*;
use alloc::vec::Vec;
use generic_array::typenum;
use hashers::pigeon::Bricolage;

macro_rules! test_enforce_same_behavior {
    (@ $iter:expr, $N:ty, $var:ident $body:block) => {{
        // arrange
        let normal_iter = ($iter);
        let peek_iter: BPeekN<_, $N> = ($iter).bpeekable::<$N>();

        // act
        let normal_result = {
            #[allow(unused_mut)]
            let mut $var = normal_iter;
            $body
        };
        let peek_result = {
            #[allow(unused_mut)]
            let mut $var = peek_iter;
            $body
        };

        // assert
        assert_eq!(normal_result, peek_result);
    }};
    ($name:ident, $iter:expr, $var:ident $body:block) => {
        #[test]
        fn $name() {
            test_enforce_same_behavior!(@ $iter, typenum::U<1>, $var $body);
            test_enforce_same_behavior!(@ $iter, typenum::U<2>, $var $body);
            test_enforce_same_behavior!(@ $iter, typenum::U<3>, $var $body);
            test_enforce_same_behavior!(@ $iter, typenum::U<4>, $var $body);
            test_enforce_same_behavior!(@ $iter, typenum::U<5>, $var $body);

            test_enforce_same_behavior!(@ $iter, typenum::U<6>, $var $body);
            test_enforce_same_behavior!(@ $iter, typenum::U<7>, $var $body);
            test_enforce_same_behavior!(@ $iter, typenum::U<8>, $var $body);
            test_enforce_same_behavior!(@ $iter, typenum::U<9>, $var $body);
            test_enforce_same_behavior!(@ $iter, typenum::U<10>, $var $body);

            test_enforce_same_behavior!(@ $iter, typenum::U<42>, $var $body);
        }
    };
}

test_enforce_same_behavior!(empty_empty, core::iter::empty::<()>(), iter {
    iter.count() == 0
});

test_enforce_same_behavior!(single_42, core::iter::once(42), iter {
    [iter.next(), iter.next()]
});

test_enforce_same_behavior!(five_ints, 1..=5, iter {
    [iter.next(), iter.next(), iter.next(), iter.next(), iter.next(), iter.next()]
});

test_enforce_same_behavior!(iter_next_back, 1..=5, iter {
    [iter.next(), iter.next(), iter.next_back(), iter.next_back(), iter.next(), iter.next_back(), iter.next()]
});

test_enforce_same_behavior!(size_next, 1..=5, iter {
    (iter.size_hint(), iter.next(), iter.size_hint(), iter.next_back(), iter.size_hint(), iter.next(), iter.next_back(), iter.size_hint())
});

test_enforce_same_behavior!(same_count, 0..100, iter {
    iter.count()
});

test_enforce_same_behavior!(same_last, -42..=43, iter {
    iter.last()
});

test_enforce_same_behavior!(same_nth, 0..100_000, iter {
    [iter.nth(1), iter.nth_back(3),iter.nth_back(1), iter.nth(42), iter.nth(2_234), iter.nth(33_222), iter.nth(999_999), iter.nth(1), iter.nth_back(1)]
});

test_enforce_same_behavior!(same_foreach, -343..=2_323, iter {
    let mut hash = Bricolage::default();

    iter.enumerate().for_each(|(no, item)|{
        hash.write_usize(no);
        hash.write_i32(item);
        hash.write_i32(item * item);
        hash.write_u128(42u128);
    });

    hash.finish()
});

test_enforce_same_behavior!(same_partition, -343..=2_323, iter {
    iter.partition::<Vec<_>, _>(|val| val % 4 == 1 || val % 7 == 4)
});

test_enforce_same_behavior!(same_reduce, 42..=2_323u32, iter {
    iter.reduce(|val1, val2| val1.wrapping_mul(val2) % (val1.wrapping_add(val2)))
});

test_enforce_same_behavior!(same_all, 42..=2_323u32, iter {
    [iter.clone().all(|v| v> 23), iter.clone().all(|v| v < 1000)]
});

test_enforce_same_behavior!(same_any, 42..=2_323u32, iter {
    [iter.clone().all(|v| v % 42 == 0), iter.clone().all(|v| v % 352_324 == 0)]
});

test_enforce_same_behavior!(same_find, 42..=2_323u32, iter {
    iter.find(|v| (v.wrapping_sub(100)) % 1_100 == 0)
});

test_enforce_same_behavior!(same_find_map, 42..=2_323u32, iter {
    #[allow(clippy::unnecessary_find_map)]
    iter.find_map(|v| ((v.wrapping_sub(100)) % 1_100 == 0).then_some(v))
});

test_enforce_same_behavior!(same_position, 42..=2_323u32, iter {
    iter.position(|v| v > 1000)
});

// TODO: test rest of the methods

#[test]
fn peek() {
    let normal_iter = 0..5;
    let mut peeked_iter = normal_iter.bpeekable3();

    let peek_1 = peeked_iter.bpeek1().expect("Must have a 1st element");
    assert_eq!(*peek_1, 0);
    // let peek0 = peek1.peek_prev(); // <-- does not compile, there's no such thing as peeking 0th element
    let peek_12 = peek_1.peek_forward().expect("Must have a 2nd element");
    assert_eq!(*peek_12, 1);
    let peek_123 = peek_12.peek_forward().expect("Must have a 3rd element");
    // let peek4 = peek3.peek_forward().expect("Must have a 4th element"); // <-- does not compile, not enough space to store 4 elements
    assert_eq!([&0, &1, &2], peek_123.peek_all());
    // assert_eq!([&0, &1], peek3.peek_all()); // <-- does not compile, exactly three elements are returned
    let peek_12 = peek_123.peek_prev(); // no need for unwrap
    assert_eq!([&0, &1], peek_12.peek_all());
    assert_eq!([0, 1], peek_12.take_all());
    // assert_eq!(1, *peek2); // <-- does not compile, `peek2` was consumed

    let peek_345 = peeked_iter
        .bpeek3()
        .expect("Must have 3rd, 4th and 5th elements");
    assert_eq!([&2, &3, &4], peek_345.peek_all());
    let peek3 = peek_345.peek_prev().peek_prev();
    assert_eq!([2], peek3.take_all());

    assert_eq!(Some([3]), peeked_iter.bpeek1().map(PeekCursor::take_all));
    assert!(
        peeked_iter.bpeek3().is_none(),
        "There are not enough elements left"
    );
    assert!(
        peeked_iter.bpeek2().is_none(),
        "There are not enough elements left"
    );
    let peek_5 = peeked_iter.bpeek1().expect("Must have 4th and 5th element");
    assert_eq!([4], peek_5.take_all());

    assert_eq!(None, peeked_iter.next());
    assert!(
        peeked_iter.bpeek3().is_none(),
        "There are not enough elements left"
    );
}
