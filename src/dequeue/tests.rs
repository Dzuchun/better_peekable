use alloc::boxed::Box;
use generic_array::typenum;
use rand::{thread_rng, Rng};

use super::Dequeue;

#[cfg(kani)]
#[derive(Debug)]
enum DequeueOperation<const MAX_IND: usize, T> {
    Create,
    PushBack(T),
    PushFront(T),
    PushBackOverwrite(T),
    PushFrontOverwrite(T),
    PopFront,
    PopBack,
    Get(usize),
    GetMut(usize),
    Len,
    Clone,
}

#[cfg(kani)]
fn kani_operation<const MAX_IND: usize, T>(
    t_generator: impl Fn() -> T,
) -> DequeueOperation<MAX_IND, T> {
    match kani::any() {
        0 => DequeueOperation::Create,
        1 => DequeueOperation::PushBack(t_generator()),
        2 => DequeueOperation::PushFront(t_generator()),
        3 => DequeueOperation::PushBackOverwrite(t_generator()),
        4 => DequeueOperation::PushFrontOverwrite(t_generator()),
        5 => DequeueOperation::PopFront,
        6 => DequeueOperation::PopBack,
        7 => DequeueOperation::Get(kani::any::<usize>() % MAX_IND),
        8 => DequeueOperation::GetMut(kani::any::<usize>() % MAX_IND),
        _ => DequeueOperation::Len,
    }
}

#[cfg(kani)]
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(1000))]
fn kani_ops_test() {
    // arrange

    use alloc::boxed::Box;
    use generic_array::typenum;

    use core::hint::black_box;

    use super::Dequeue;
    let mut dequeue = Dequeue::<Box<u8>, typenum::U<10>>::new();

    for _ in 0..10 {
        let op = kani_operation::<20, _>(|| Box::new(kani::any::<u8>()));

        match op {
            DequeueOperation::Create => dequeue = Dequeue::new(),
            DequeueOperation::PushBack(item) => {
                black_box(dequeue.push_back(item));
            }
            DequeueOperation::PushFront(item) => {
                black_box(dequeue.push_front(item));
            }
            DequeueOperation::PushBackOverwrite(item) => {
                black_box(dequeue.push_back_overwrite(item));
            }
            DequeueOperation::PushFrontOverwrite(item) => {
                black_box(dequeue.push_front_overwrite(item));
            }
            DequeueOperation::PopFront => {
                black_box(dequeue.pop_front());
            }
            DequeueOperation::PopBack => {
                black_box(dequeue.pop_back());
            }
            DequeueOperation::Get(index) => {
                black_box(dequeue.get(index));
            }
            DequeueOperation::GetMut(index) => {
                black_box(dequeue.get_mut(index));
            }
            DequeueOperation::Len => {
                black_box(dequeue.len());
            }
            DequeueOperation::Clone => unreachable!(),
        }
    }
}

#[test]
fn create_drop() {
    let _ = Dequeue::<Box<u8>, typenum::U<10>>::new();
}

#[test]
fn push_pop() {
    let mut dequeue = Dequeue::<Box<u8>, typenum::U<10>>::new();

    dequeue.push_back(Box::new(0)).assert();
    dequeue.push_back(Box::new(0)).assert();
    dequeue.push_back(Box::new(0)).assert();

    assert_eq!(dequeue.len(), 3);

    let _ = dequeue.pop_back().unwrap();
    let _ = dequeue.pop_front().unwrap();

    assert_eq!(dequeue.len(), 1);
}

#[test]
fn push_pop_overwrite() {
    let mut dequeue = Dequeue::<Box<u8>, typenum::U<10>>::new();

    dequeue.push_back_overwrite(Box::new(0));
    dequeue.push_back_overwrite(Box::new(0));
    dequeue.push_back_overwrite(Box::new(0));

    assert_eq!(dequeue.len(), 3);

    let _ = dequeue.pop_back().unwrap();
    let _ = dequeue.pop_front().unwrap();

    assert_eq!(dequeue.len(), 1);
}

#[test]
fn pop_push() {
    let mut dequeue = Dequeue::<Box<u8>, typenum::U<10>>::new();

    dequeue.pop_back().ok_or(()).unwrap_err();
    dequeue.pop_front().ok_or(()).unwrap_err();
    dequeue.pop_front().ok_or(()).unwrap_err();
    dequeue.pop_back().ok_or(()).unwrap_err();

    assert_eq!(dequeue.len(), 0);

    dequeue.push_front(Box::new(0)).assert();
    dequeue.push_back(Box::new(0)).assert();
    dequeue.push_front(Box::new(0)).assert();

    assert_eq!(dequeue.len(), 3);

    let _ = dequeue.pop_back().unwrap();
    let _ = dequeue.pop_front().unwrap();

    assert_eq!(dequeue.len(), 1);
}

#[test]
fn pop_push_overwrite() {
    let mut dequeue = Dequeue::<Box<u8>, typenum::U<10>>::new();

    dequeue.pop_back().ok_or(()).unwrap_err();
    dequeue.pop_front().ok_or(()).unwrap_err();
    dequeue.pop_front().ok_or(()).unwrap_err();
    dequeue.pop_back().ok_or(()).unwrap_err();

    assert_eq!(dequeue.len(), 0);

    dequeue.push_front_overwrite(Box::new(0));
    dequeue.push_back_overwrite(Box::new(0));
    dequeue.push_front_overwrite(Box::new(0));

    assert_eq!(dequeue.len(), 3);

    let _ = dequeue.pop_back().unwrap();
    let _ = dequeue.pop_front().unwrap();

    assert_eq!(dequeue.len(), 1);
}

#[test]
fn clone_drop() {
    let dequeue = Dequeue::<Box<u8>, typenum::U<10>>::new();

    let _dequeue = dequeue.clone();
}

#[test]
fn clone_drop2() {
    let mut dequeue = Dequeue::<Box<u8>, typenum::U<10>>::new();

    dequeue.push_back(Box::new(1)).assert();
    // [1]
    dequeue.push_front(Box::new(2)).assert();
    // [2, 1]
    dequeue.push_back_overwrite(Box::new(3));
    // [2, 1, 3]
    dequeue.push_front_overwrite(Box::new(4));
    // [4, 2, 1, 3]

    assert_eq!(dequeue.len(), 4);

    dequeue = dequeue.clone();

    assert_eq!(dequeue.len(), 4);

    assert_eq!(*dequeue.pop_back().unwrap(), 3);
    assert_eq!(*dequeue.pop_back().unwrap(), 1);
    assert_eq!(*dequeue.pop_back().unwrap(), 2);

    assert_eq!(dequeue.len(), 1);
}

#[test]
fn clone_drop3() {
    let mut dequeue = Dequeue::<Box<u8>, typenum::U<10>>::new();

    dequeue.push_back(Box::new(1)).assert();
    assert_eq!(*dequeue.start, 0);
    assert_eq!(*dequeue.len, 1);
    // [1]
    assert_eq!(*dequeue[0], 1);

    dequeue.push_front(Box::new(2)).assert();
    assert_eq!(*dequeue.start, 9);
    assert_eq!(*dequeue.len, 2);
    // [2, 1]
    assert_eq!(*dequeue[0], 2);
    assert_eq!(*dequeue[1], 1);

    dequeue.push_back_overwrite(Box::new(3));
    assert_eq!(*dequeue.start, 9);
    assert_eq!(*dequeue.len, 3);
    // [2, 1, 3]
    assert_eq!(*dequeue[0], 2);
    assert_eq!(*dequeue[1], 1);
    assert_eq!(*dequeue[2], 3);

    dequeue.push_front_overwrite(Box::new(4));
    assert_eq!(*dequeue.start, 8);
    assert_eq!(*dequeue.len, 4);
    // [4, 2, 1, 3]
    assert_eq!(*dequeue[0], 4);
    assert_eq!(*dequeue[1], 2);
    assert_eq!(*dequeue[2], 1);
    assert_eq!(*dequeue[3], 3);

    assert_eq!(dequeue.len(), 4);

    dequeue = dequeue.clone();
    dequeue = dequeue.clone();

    assert_eq!(dequeue.len(), 4);

    assert_eq!(*dequeue.pop_back().unwrap(), 3);
    assert_eq!(*dequeue.pop_back().unwrap(), 1);
    assert_eq!(*dequeue.pop_back().unwrap(), 2);
    assert_eq!(*dequeue.pop_back().unwrap(), 4);

    assert_eq!(dequeue.len(), 0);

    drop(dequeue);
}

#[test]
fn overwrite() {
    let mut rand = thread_rng();

    let mut dequeue = Dequeue::<Box<u8>, typenum::U<10>>::new();

    for _ in 0..5 {
        for i in 0..10 {
            assert_eq!(dequeue.len(), i);
            if rand.gen_bool(0.224) {
                dequeue.push_back(Box::new(0)).assert();
            } else {
                dequeue.push_front(Box::new(0)).assert();
            }
            assert_eq!(dequeue.len(), i + 1);
        }
        for _ in 0..20 {
            assert_eq!(dequeue.len(), 10);
            if rand.gen_bool(0.4442) {
                dequeue.push_back_overwrite(Box::new(0));
            } else {
                dequeue.push_front_overwrite(Box::new(0));
            }
        }

        for i in (1..=10).rev() {
            assert_eq!(dequeue.len(), i);
            if rand.gen_bool(0.6545) {
                dequeue.pop_back().unwrap();
            } else {
                dequeue.pop_front().unwrap();
            }
            assert_eq!(dequeue.len(), i - 1);
        }

        if rand.gen_bool(0.764) {
            dequeue = dequeue.clone();
        }
    }
}

#[test]
fn ops_test() {
    use alloc::boxed::Box;
    use generic_array::typenum;

    use super::Dequeue;

    let mut rand = thread_rng();
    let mut dequeue = Dequeue::<Box<u8>, typenum::U<10>>::new();

    for _ in 0..1000 {
        match rand.gen_range(0..11) {
            0 => dequeue = Dequeue::new(),
            1 => {
                let _ = dequeue.push_back(Box::new(rand.gen()));
            }
            2 => {
                let _ = dequeue.push_front(Box::new(rand.gen()));
            }
            3 => {
                dequeue.push_back_overwrite(Box::new(rand.gen()));
            }
            4 => {
                dequeue.push_front_overwrite(Box::new(rand.gen()));
            }
            5 => {
                dequeue.pop_back();
            }
            6 => {
                dequeue.pop_front();
            }
            7 => {
                dequeue.get(rand.gen_range(0..20));
            }
            8 => {
                dequeue.get_mut(rand.gen_range(0..20));
            }
            9 => {
                dequeue.len();
            }
            10 => dequeue = dequeue.clone(),
            11 => dequeue.clear(),
            _ => unreachable!(),
        }
    }
}
