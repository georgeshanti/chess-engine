//! Dynamic array initialisation is very dangerous currently.
//! The safest way is to initialize one with a default value
//!
//! ```
//! let mut array = [0; 32];
//! for i in 0..32 {
//!     array[i] = i;
//! }
//! ```
//!
//! This is not possible in general though. For any type `[T; N]`,
//! T either needs to be [`Copy`], or there needs to be a `const t: T`.
//! This is definitely not always the case.
//!
//! The second problem is efficiency. In the example above, we are
//! filling an array with zeros, just to replace them. While the
//! compiler can sometimes optimise this away, it's nice to have the guarantee.
//!
//! So, what's the alternative? How about [`MaybeUninit`]! Although, it's not that simple.
//! Take the following example, which uses completely safe Rust! Can you spot the error?
//!
//! ```should_panic
//! # #![feature(maybe_uninit_uninit_array)]
//! # #![feature(maybe_uninit_extra)]
//! # use std::mem::MaybeUninit;
//! let mut uninit: [MaybeUninit<String>; 8] = MaybeUninit::uninit_array();
//! uninit[0].write("foo".to_string());
//! uninit[1].write("bar".to_string());
//! uninit[2].write("baz".to_string());
//! panic!("oops");
//! ```
//!
//! Did you spot it? Right there is a memory leak. The key here is that
//! [`MaybeUninit`] **does not** implement [`Drop`]. This makes sense
//! since the value could be uninitialized, and calling [`Drop`] on an
//! uninitialized value is undefined behaviour. The result of this is that
//! the 3 [`String`] values we did initialize never got dropped!
//! Now, this is safe according to Rust. Leaking memory is not undefined
//! behaviour. But it's still not something we should promote.
//!
//! What other options do we have? The only solution is to provide a new
//! `struct` that wraps the array, and properly implements [`Drop`]. That
//! way, if `drop` is called, we can make sure any initialized values get
//! dropped properly. This is exactly what [`ArrayBuilder`] provides.
//!
//! ```should_panic
//! use array_builder::ArrayBuilder;
//! let mut uninit: ArrayBuilder<String, 8> = ArrayBuilder::new();
//! uninit.push("foo".to_string());
//! uninit.push("bar".to_string());
//! uninit.push("baz".to_string());
//! panic!("oops"); // ArrayBuilder drops the 3 values above for you
//! ```
//!
//! ```
//! use array_builder::ArrayBuilder;
//! let mut uninit: ArrayBuilder<String, 3> = ArrayBuilder::new();
//! uninit.push("foo".to_string());
//! uninit.push("bar".to_string());
//! uninit.push("baz".to_string());
//! let array: [String; 3] = uninit.build().unwrap();
//! ```
//!
//! You can also take a peek at what the current set of initialised values are
//!
//! ```
//! use array_builder::ArrayBuilder;
//! let mut uninit: ArrayBuilder<usize, 4> = ArrayBuilder::new();
//! uninit.push(1);
//! uninit.push(2);
//! uninit.push(3);
//!
//! // we can't build just yet
//! let mut uninit = uninit.build().unwrap_err();
//! let slice: &[usize] = &uninit;
//! assert_eq!(&[1, 2, 3], slice);
//!
//! uninit.push(4);
//! assert_eq!([1, 2, 3, 4], uninit.build().unwrap());
//! ```

use core::{
    cmp, fmt,
    mem::{self, ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr, slice,
};
use std::{cell::{UnsafeCell, SyncUnsafeCell}, fmt::Display, sync::{Mutex, RwLock}};

use serde::de::value;

use crate::log;

/// ArrayBuilder makes it easy to dynamically build arrays safely
/// and efficiently.
///
/// ```
/// use array_builder::ArrayBuilder;
///
/// struct ArrayIterator<I: Iterator, const N: usize> {
///     builder: ArrayBuilder<I::Item, N>,
///     iter: I,
/// }
///
/// impl<I: Iterator, const N: usize> Iterator for ArrayIterator<I, N> {
///     type Item = [I::Item; N];
///
///     fn next(&mut self) -> Option<Self::Item> {
///         for _ in self.builder.len()..N {
///             self.builder.push(self.iter.next()?);
///         }
///         self.builder.take().build().ok()
///     }
/// }
///
/// impl<I: Iterator, const N: usize> ArrayIterator<I, N> {
///     pub fn new(i: impl IntoIterator<IntoIter=I>) -> Self {
///         Self {
///             builder: ArrayBuilder::new(),
///             iter: i.into_iter(),
///         }
///     }
///
///     pub fn remaining(&self) -> &[I::Item] {
///         &self.builder
///     }
/// }
///
/// let mut i = ArrayIterator::new(0..10);
/// assert_eq!(Some([0, 1, 2, 3]), i.next());
/// assert_eq!(Some([4, 5, 6, 7]), i.next());
/// assert_eq!(None, i.next());
/// assert_eq!(&[8, 9], i.remaining());
/// ```
pub struct ConcurrentQueuePage<T, const N: usize> {
    buf: [SyncUnsafeCell<MaybeUninit<T>>; N],
}

// impl<T: fmt::Debug, const N: usize> fmt::Debug for ConcurrentQueuePage<T, N> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.debug_struct("ArrayBuilder")
//             .field("capacity", &N)
//             .field("length", &self.len)
//             .field("values", &self.deref())
//             .finish()
//     }
// }

// impl<T, const N: usize> Drop for ConcurrentQueuePage<T, N> {
//     fn drop(&mut self) {
//         self.clear()
//     }
// }

// impl<T, const N: usize> Deref for ConcurrentQueuePage<T, N> {
//     type Target = [T];
//     fn deref(&self) -> &[T] {
//         unsafe { slice::from_raw_parts(self.as_ptr(), *self.len.read().unwrap()) }
//     }
// }

// impl<T, const N: usize> DerefMut for ConcurrentQueuePage<T, N> {
//     fn deref_mut(&mut self) -> &mut [T] {
//         unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), *self.len.read().unwrap()) }
//     }
// }

impl<T, const N: usize> ConcurrentQueuePage<T, N> {
    const UNINIT: SyncUnsafeCell<MaybeUninit<T>> = SyncUnsafeCell::new(MaybeUninit::uninit());

    /// Create a new ArrayBuilder, backed by an uninitialised [T; N]
    pub fn new() -> Self {
        Self {
            buf: [Self::UNINIT; N],
        }
    }

    fn as_ptr(&self) -> *mut T {
        unsafe {
            self.buf[0].get() as *mut T
        }
    }

    pub unsafe fn write(&self, source: &[T], from: usize, to: usize) {
        assert!(from<=to);
        assert!(from<N);
        assert!(to<=N);
        // log!("Mem address: {:p}", self.as_ptr().add(from));
        // log!("Writing to Queue page from {} to {}", from, to);
        // log!("source: {}", source[0]);
        unsafe {
            ptr::copy(source.as_ptr(), self.as_ptr().add(from), to - from);
        };
        // ptr::copy_nonoverlapping(source.as_ptr(), self.as_ptr().add(from), to - from);

        // log!("source: {}", *self.as_ptr().add(from));
    }

    pub unsafe fn read<const O: usize>(&self, destination: &mut [T; O], from: usize, to: usize) {
        assert!(from<=to);
        if from >= N {
            // log!("Going to fail: {}", from);
        }
        assert!(from<N);
        assert!(to<=N);
        // log!("Mem address: {:p}", self.as_ptr().add(from));
        // log!("Reading to Queue page from {} to {}", from, to);
        ptr::copy(self.as_ptr().add(from), destination.as_mut_ptr(), to - from);
    }
}
