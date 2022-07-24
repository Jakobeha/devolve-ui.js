//! Reference-counted pointers which are guaranteed not to outlive a certain lifetime.
//! Thus they can have lifetime data, whereas `SRc`'s data must be `'static`).
//! These may also be determined strong or weak at runtime instead of compile-time.

use std::cell::Cell;
use std::marker::{PhantomData, Unsize};
use std::ops::{CoerceUnsized, DispatchFromDyn};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::ptr::NonNull;

// Most of this code is copied from SRc in the standard library

/// Scoped, strong or weak reference-counted pointer.
pub struct SRc<'a, T: ?Sized> {
    ptr: NonNull<SRcBox<'a, T>>,
}

pub struct SRcBox<'a, T: ?Sized> {
    strong: Cell<usize>,
    weak: Cell<usize>,
    value: T,
}

impl<'a, T: ?Sized> !Send for SRc<'a, T> {}

// Note that this negative impl isn't strictly necessary for correctness,
// as `SRc` transitively contains a `Cell`, which is itself `!Sync`.
// However, given how important `SRc`'s `!Sync`-ness is,
// having an explicit negative impl is nice for documentation purposes
// and results in nicer error messages.
impl<'a, T: ?Sized> !Sync for SRc<'a, T> {}

impl<'a, T: RefUnwindSafe + ?Sized> UnwindSafe for SRc<'a, T> {}
impl<'a, T: RefUnwindSafe + ?Sized> RefUnwindSafe for SRc<'a, T> {}

impl<'a, T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<SRc<'a, U>> for SRc<'a, T> {}
impl<'a, T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<SRc<'a, U>> for SRc<'a, T> {}

impl<'a, T: ?Sized> SRc<'a, T> {
    #[inline(always)]
    fn inner(&self) -> &SRcBox<'a, T> {
        // This unsafety is ok because while this SRc is alive we're guaranteed
        // that the inner pointer is valid.
        unsafe { self.ptr.as_ref() }
    }

    unsafe fn from_inner(ptr: NonNull<SRcBox<'a, T>>) -> Self {
        Self { ptr }
    }

    unsafe fn from_ptr(ptr: *mut SRcBox<'a, T>) -> Self {
        Self::from_inner(NonNull::new_unchecked(ptr))
    }
}

impl<'a, T> SRc<'a, T> {
    /// Constructs a new `SRc<T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let five = SRc::new(5);
    /// ```
    pub fn new(value: T) -> SRc<'a, T> {
        // There is an implicit weak pointer owned by all the strong
        // pointers, which ensures that the weak destructor never frees
        // the allocation while the strong destructor is running, even
        // if the weak pointer is stored inside the strong one.
        unsafe {
            Self::from_inner(
                Box::leak(Box::new(SRcBox { strong: Cell::new(1), weak: Cell::new(1), value }))
                    .into(),
            )
        }
    }

    /// Constructs a new `SRc<T>` while giving you a `Weak<T>` to the allocation,
    /// to allow you to construct a `T` which holds a weak pointer to itself.
    ///
    /// Generally, a structure circularly referencing itself, either directly or
    /// indirectly, should not hold a strong reference to itself to prevent a memory leak.
    /// Using this function, you get access to the weak pointer during the
    /// initialization of `T`, before the `SRc<T>` is created, such that you can
    /// clone and store it inside the `T`.
    ///
    /// `new_cyclic` first allocates the managed allocation for the `SRc<T>`,
    /// then calls your closure, giving it a `Weak<T>` to this allocation,
    /// and only afterwards completes the construction of the `SRc<T>` by placing
    /// the `T` returned from your closure into the allocation.
    ///
    /// Since the new `SRc<T>` is not fully-constructed until `SRc<T>::new_cyclic`
    /// returns, calling [`upgrade`] on the weak reference inside your closure will
    /// fail and result in a `None` value.
    ///
    /// # Panics
    ///
    /// If `data_fn` panics, the panic is propagated to the caller, and the
    /// temporary [`Weak<T>`] is dropped normally.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![allow(dead_code)]
    /// use std::rc::{SRc, Weak};
    ///
    /// struct Gadget {
    ///     me: Weak<Gadget>,
    /// }
    ///
    /// impl Gadget {
    ///     /// Construct a reference counted Gadget.
    ///     fn new() -> SRc<Self> {
    ///         // `me` is a `Weak<Gadget>` pointing at the new allocation of the
    ///         // `SRc` we're constructing.
    ///         SRc::new_cyclic(|me| {
    ///             // Create the actual struct here.
    ///             Gadget { me: me.clone() }
    ///         })
    ///     }
    ///
    ///     /// Return a reference counted pointer to Self.
    ///     fn me(&self) -> SRc<Self> {
    ///         self.me.upgrade().unwrap()
    ///     }
    /// }
    /// ```
    /// [`upgrade`]: Weak::upgrade
    #[cfg(not(no_global_oom_handling))]
    #[stable(feature = "arc_new_cyclic", since = "1.60.0")]
    pub fn new_cyclic<F>(data_fn: F) -> SRc<T>
        where
            F: FnOnce(&Weak<T>) -> T,
    {
        // Construct the inner in the "uninitialized" state with a single
        // weak reference.
        let uninit_ptr: NonNull<_> = Box::leak(Box::new(SRcBox {
            strong: Cell::new(0),
            weak: Cell::new(1),
            value: mem::MaybeUninit::<T>::uninit(),
        }))
            .into();

        let init_ptr: NonNull<SRcBox<T>> = uninit_ptr.cast();

        let weak = Weak { ptr: init_ptr };

        // It's important we don't give up ownership of the weak pointer, or
        // else the memory might be freed by the time `data_fn` returns. If
        // we really wanted to pass ownership, we could create an additional
        // weak pointer for ourselves, but this would result in additional
        // updates to the weak reference count which might not be necessary
        // otherwise.
        let data = data_fn(&weak);

        let strong = unsafe {
            let inner = init_ptr.as_ptr();
            ptr::write(ptr::addr_of_mut!((*inner).value), data);

            let prev_value = (*inner).strong.get();
            debug_assert_eq!(prev_value, 0, "No prior strong references should exist");
            (*inner).strong.set(1);

            SRc::from_inner(init_ptr)
        };

        // Strong references should collectively own a shared weak reference,
        // so don't run the destructor for our old weak reference.
        mem::forget(weak);
        strong
    }

    /// Constructs a new `SRc` with uninitialized contents.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(new_uninit)]
    /// #![feature(get_mut_unchecked)]
    ///
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let mut five = SRc::<u32>::new_uninit();
    ///
    /// // Deferred initialization:
    /// SRc::get_mut(&mut five).unwrap().write(5);
    ///
    /// let five = unsafe { five.assume_init() };
    ///
    /// assert_eq!(*five, 5)
    /// ```
    #[cfg(not(no_global_oom_handling))]
    #[unstable(feature = "new_uninit", issue = "63291")]
    #[must_use]
    pub fn new_uninit() -> SRc<mem::MaybeUninit<T>> {
        unsafe {
            SRc::from_ptr(SRc::allocate_for_layout(
                Layout::new::<T>(),
                |layout| Global.allocate(layout),
                |mem| mem as *mut SRcBox<mem::MaybeUninit<T>>,
            ))
        }
    }

    /// Constructs a new `SRc` with uninitialized contents, with the memory
    /// being filled with `0` bytes.
    ///
    /// See [`MaybeUninit::zeroed`][zeroed] for examples of correct and
    /// incorrect usage of this method.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(new_uninit)]
    ///
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let zero = SRc::<u32>::new_zeroed();
    /// let zero = unsafe { zero.assume_init() };
    ///
    /// assert_eq!(*zero, 0)
    /// ```
    ///
    /// [zeroed]: mem::MaybeUninit::zeroed
    #[cfg(not(no_global_oom_handling))]
    #[unstable(feature = "new_uninit", issue = "63291")]
    #[must_use]
    pub fn new_zeroed() -> SRc<mem::MaybeUninit<T>> {
        unsafe {
            SRc::from_ptr(SRc::allocate_for_layout(
                Layout::new::<T>(),
                |layout| Global.allocate_zeroed(layout),
                |mem| mem as *mut SRcBox<mem::MaybeUninit<T>>,
            ))
        }
    }

    /// Constructs a new `SRc<T>`, returning an error if the allocation fails
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let five = SRc::try_new(5);
    /// # Ok::<(), std::alloc::AllocError>(())
    /// ```
    #[unstable(feature = "allocator_api", issue = "32838")]
    pub fn try_new(value: T) -> Result<SRc<T>, AllocError> {
        // There is an implicit weak pointer owned by all the strong
        // pointers, which ensures that the weak destructor never frees
        // the allocation while the strong destructor is running, even
        // if the weak pointer is stored inside the strong one.
        unsafe {
            Ok(Self::from_inner(
                Box::leak(Box::try_new(SRcBox { strong: Cell::new(1), weak: Cell::new(1), value })?)
                    .into(),
            ))
        }
    }

    /// Constructs a new `SRc` with uninitialized contents, returning an error if the allocation fails
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api, new_uninit)]
    /// #![feature(get_mut_unchecked)]
    ///
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let mut five = SRc::<u32>::try_new_uninit()?;
    ///
    /// // Deferred initialization:
    /// SRc::get_mut(&mut five).unwrap().write(5);
    ///
    /// let five = unsafe { five.assume_init() };
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), std::alloc::AllocError>(())
    /// ```
    #[unstable(feature = "allocator_api", issue = "32838")]
    // #[unstable(feature = "new_uninit", issue = "63291")]
    pub fn try_new_uninit() -> Result<SRc<mem::MaybeUninit<T>>, AllocError> {
        unsafe {
            Ok(SRc::from_ptr(SRc::try_allocate_for_layout(
                Layout::new::<T>(),
                |layout| Global.allocate(layout),
                |mem| mem as *mut SRcBox<mem::MaybeUninit<T>>,
            )?))
        }
    }

    /// Constructs a new `SRc` with uninitialized contents, with the memory
    /// being filled with `0` bytes, returning an error if the allocation fails
    ///
    /// See [`MaybeUninit::zeroed`][zeroed] for examples of correct and
    /// incorrect usage of this method.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api, new_uninit)]
    ///
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let zero = SRc::<u32>::try_new_zeroed()?;
    /// let zero = unsafe { zero.assume_init() };
    ///
    /// assert_eq!(*zero, 0);
    /// # Ok::<(), std::alloc::AllocError>(())
    /// ```
    ///
    /// [zeroed]: mem::MaybeUninit::zeroed
    #[unstable(feature = "allocator_api", issue = "32838")]
    //#[unstable(feature = "new_uninit", issue = "63291")]
    pub fn try_new_zeroed() -> Result<SRc<mem::MaybeUninit<T>>, AllocError> {
        unsafe {
            Ok(SRc::from_ptr(SRc::try_allocate_for_layout(
                Layout::new::<T>(),
                |layout| Global.allocate_zeroed(layout),
                |mem| mem as *mut SRcBox<mem::MaybeUninit<T>>,
            )?))
        }
    }
    /// Constructs a new `Pin<SRc<T>>`. If `T` does not implement `Unpin`, then
    /// `value` will be pinned in memory and unable to be moved.
    #[cfg(not(no_global_oom_handling))]
    #[stable(feature = "pin", since = "1.33.0")]
    #[must_use]
    pub fn pin(value: T) -> Pin<SRc<T>> {
        unsafe { Pin::new_unchecked(SRc::new(value)) }
    }

    /// Returns the inner value, if the `SRc` has exactly one strong reference.
    ///
    /// Otherwise, an [`Err`] is returned with the same `SRc` that was
    /// passed in.
    ///
    /// This will succeed even if there are outstanding weak references.
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let x = SRc::new(3);
    /// assert_eq!(SRc::try_unwrap(x), Ok(3));
    ///
    /// let x = SRc::new(4);
    /// let _y = SRc::clone(&x);
    /// assert_eq!(*SRc::try_unwrap(x).unwrap_err(), 4);
    /// ```
    #[inline]
    #[stable(feature = "rc_unique", since = "1.4.0")]
    pub fn try_unwrap(this: Self) -> Result<T, Self> {
        if SRc::strong_count(&this) == 1 {
            unsafe {
                let val = ptr::read(&*this); // copy the contained object

                // Indicate to Weaks that they can't be promoted by decrementing
                // the strong count, and then remove the implicit "strong weak"
                // pointer while also handling drop logic by just crafting a
                // fake Weak.
                this.inner().dec_strong();
                let _weak = Weak { ptr: this.ptr };
                forget(this);
                Ok(val)
            }
        } else {
            Err(this)
        }
    }
}

impl<T> SRc<[T]> {
    /// Constructs a new reference-counted slice with uninitialized contents.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(new_uninit)]
    /// #![feature(get_mut_unchecked)]
    ///
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let mut values = SRc::<[u32]>::new_uninit_slice(3);
    ///
    /// // Deferred initialization:
    /// let data = SRc::get_mut(&mut values).unwrap();
    /// data[0].write(1);
    /// data[1].write(2);
    /// data[2].write(3);
    ///
    /// let values = unsafe { values.assume_init() };
    ///
    /// assert_eq!(*values, [1, 2, 3])
    /// ```
    #[cfg(not(no_global_oom_handling))]
    #[unstable(feature = "new_uninit", issue = "63291")]
    #[must_use]
    pub fn new_uninit_slice(len: usize) -> SRc<[mem::MaybeUninit<T>]> {
        unsafe { SRc::from_ptr(SRc::allocate_for_slice(len)) }
    }

    /// Constructs a new reference-counted slice with uninitialized contents, with the memory being
    /// filled with `0` bytes.
    ///
    /// See [`MaybeUninit::zeroed`][zeroed] for examples of correct and
    /// incorrect usage of this method.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(new_uninit)]
    ///
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let values = SRc::<[u32]>::new_zeroed_slice(3);
    /// let values = unsafe { values.assume_init() };
    ///
    /// assert_eq!(*values, [0, 0, 0])
    /// ```
    ///
    /// [zeroed]: mem::MaybeUninit::zeroed
    #[cfg(not(no_global_oom_handling))]
    #[unstable(feature = "new_uninit", issue = "63291")]
    #[must_use]
    pub fn new_zeroed_slice(len: usize) -> SRc<[mem::MaybeUninit<T>]> {
        unsafe {
            SRc::from_ptr(SRc::allocate_for_layout(
                Layout::array::<T>(len).unwrap(),
                |layout| Global.allocate_zeroed(layout),
                |mem| {
                    ptr::slice_from_raw_parts_mut(mem as *mut T, len)
                        as *mut SRcBox<[mem::MaybeUninit<T>]>
                },
            ))
        }
    }
}

impl<T> SRc<mem::MaybeUninit<T>> {
    /// Converts to `SRc<T>`.
    ///
    /// # Safety
    ///
    /// As with [`MaybeUninit::assume_init`],
    /// it is up to the caller to guarantee that the inner value
    /// really is in an initialized state.
    /// Calling this when the content is not yet fully initialized
    /// causes immediate undefined behavior.
    ///
    /// [`MaybeUninit::assume_init`]: mem::MaybeUninit::assume_init
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(new_uninit)]
    /// #![feature(get_mut_unchecked)]
    ///
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let mut five = SRc::<u32>::new_uninit();
    ///
    /// // Deferred initialization:
    /// SRc::get_mut(&mut five).unwrap().write(5);
    ///
    /// let five = unsafe { five.assume_init() };
    ///
    /// assert_eq!(*five, 5)
    /// ```
    #[unstable(feature = "new_uninit", issue = "63291")]
    #[inline]
    pub unsafe fn assume_init(self) -> SRc<T> {
        unsafe { SRc::from_inner(mem::ManuallyDrop::new(self).ptr.cast()) }
    }
}

impl<T> SRc<[mem::MaybeUninit<T>]> {
    /// Converts to `SRc<[T]>`.
    ///
    /// # Safety
    ///
    /// As with [`MaybeUninit::assume_init`],
    /// it is up to the caller to guarantee that the inner value
    /// really is in an initialized state.
    /// Calling this when the content is not yet fully initialized
    /// causes immediate undefined behavior.
    ///
    /// [`MaybeUninit::assume_init`]: mem::MaybeUninit::assume_init
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(new_uninit)]
    /// #![feature(get_mut_unchecked)]
    ///
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let mut values = SRc::<[u32]>::new_uninit_slice(3);
    ///
    /// // Deferred initialization:
    /// let data = SRc::get_mut(&mut values).unwrap();
    /// data[0].write(1);
    /// data[1].write(2);
    /// data[2].write(3);
    ///
    /// let values = unsafe { values.assume_init() };
    ///
    /// assert_eq!(*values, [1, 2, 3])
    /// ```
    #[unstable(feature = "new_uninit", issue = "63291")]
    #[inline]
    pub unsafe fn assume_init(self) -> SRc<[T]> {
        unsafe { SRc::from_ptr(mem::ManuallyDrop::new(self).ptr.as_ptr() as _) }
    }
}

impl<T: ?Sized> SRc<T> {
    /// Consumes the `SRc`, returning the wrapped pointer.
    ///
    /// To avoid a memory leak the pointer must be converted back to an `SRc` using
    /// [`SRc::from_raw`].
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let x = SRc::new("hello".to_owned());
    /// let x_ptr = SRc::into_raw(x);
    /// assert_eq!(unsafe { &*x_ptr }, "hello");
    /// ```
    #[stable(feature = "rc_raw", since = "1.17.0")]
    pub fn into_raw(this: Self) -> *const T {
        let ptr = Self::as_ptr(&this);
        mem::forget(this);
        ptr
    }

    /// Provides a raw pointer to the data.
    ///
    /// The counts are not affected in any way and the `SRc` is not consumed. The pointer is valid
    /// for as long there are strong counts in the `SRc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let x = SRc::new("hello".to_owned());
    /// let y = SRc::clone(&x);
    /// let x_ptr = SRc::as_ptr(&x);
    /// assert_eq!(x_ptr, SRc::as_ptr(&y));
    /// assert_eq!(unsafe { &*x_ptr }, "hello");
    /// ```
    #[stable(feature = "weak_into_raw", since = "1.45.0")]
    pub fn as_ptr(this: &Self) -> *const T {
        let ptr: *mut SRcBox<T> = NonNull::as_ptr(this.ptr);

        // SAFETY: This cannot go through Deref::deref or SRc::inner because
        // this is required to retain raw/mut provenance such that e.g. `get_mut` can
        // write through the pointer after the SRc is recovered through `from_raw`.
        unsafe { ptr::addr_of_mut!((*ptr).value) }
    }

    /// Constructs an `SRc<T>` from a raw pointer.
    ///
    /// The raw pointer must have been previously returned by a call to
    /// [`SRc<U>::into_raw`][into_raw] where `U` must have the same size
    /// and alignment as `T`. This is trivially true if `U` is `T`.
    /// Note that if `U` is not `T` but has the same size and alignment, this is
    /// basically like transmuting references of different types. See
    /// [`mem::transmute`] for more information on what
    /// restrictions apply in this case.
    ///
    /// The user of `from_raw` has to make sure a specific value of `T` is only
    /// dropped once.
    ///
    /// This function is unsafe because improper use may lead to memory unsafety,
    /// even if the returned `SRc<T>` is never accessed.
    ///
    /// [into_raw]: SRc::into_raw
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let x = SRc::new("hello".to_owned());
    /// let x_ptr = SRc::into_raw(x);
    ///
    /// unsafe {
    ///     // Convert back to an `SRc` to prevent leak.
    ///     let x = SRc::from_raw(x_ptr);
    ///     assert_eq!(&*x, "hello");
    ///
    ///     // Further calls to `SRc::from_raw(x_ptr)` would be memory-unsafe.
    /// }
    ///
    /// // The memory was freed when `x` went out of scope above, so `x_ptr` is now dangling!
    /// ```
    #[stable(feature = "rc_raw", since = "1.17.0")]
    pub unsafe fn from_raw(ptr: *const T) -> Self {
        let offset = unsafe { data_offset(ptr) };

        // Reverse the offset to find the original SRcBox.
        let rc_ptr =
            unsafe { (ptr as *mut u8).offset(-offset).with_metadata_of(ptr as *mut SRcBox<T>) };

        unsafe { Self::from_ptr(rc_ptr) }
    }

    /// Creates a new [`Weak`] pointer to this allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let five = SRc::new(5);
    ///
    /// let weak_five = SRc::downgrade(&five);
    /// ```
    #[must_use = "this returns a new `Weak` pointer, \
                  without modifying the original `SRc`"]
    #[stable(feature = "rc_weak", since = "1.4.0")]
    pub fn downgrade(this: &Self) -> Weak<T> {
        this.inner().inc_weak();
        // Make sure we do not create a dangling Weak
        debug_assert!(!is_dangling(this.ptr.as_ptr()));
        Weak { ptr: this.ptr }
    }

    /// Gets the number of [`Weak`] pointers to this allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let five = SRc::new(5);
    /// let _weak_five = SRc::downgrade(&five);
    ///
    /// assert_eq!(1, SRc::weak_count(&five));
    /// ```
    #[inline]
    #[stable(feature = "rc_counts", since = "1.15.0")]
    pub fn weak_count(this: &Self) -> usize {
        this.inner().weak() - 1
    }

    /// Gets the number of strong (`SRc`) pointers to this allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let five = SRc::new(5);
    /// let _also_five = SRc::clone(&five);
    ///
    /// assert_eq!(2, SRc::strong_count(&five));
    /// ```
    #[inline]
    #[stable(feature = "rc_counts", since = "1.15.0")]
    pub fn strong_count(this: &Self) -> usize {
        this.inner().strong()
    }

    /// Increments the strong reference count on the `SRc<T>` associated with the
    /// provided pointer by one.
    ///
    /// # Safety
    ///
    /// The pointer must have been obtained through `SRc::into_raw`, and the
    /// associated `SRc` instance must be valid (i.e. the strong count must be at
    /// least 1) for the duration of this method.
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let five = SRc::new(5);
    ///
    /// unsafe {
    ///     let ptr = SRc::into_raw(five);
    ///     SRc::increment_strong_count(ptr);
    ///
    ///     let five = SRc::from_raw(ptr);
    ///     assert_eq!(2, SRc::strong_count(&five));
    /// }
    /// ```
    #[inline]
    #[stable(feature = "rc_mutate_strong_count", since = "1.53.0")]
    pub unsafe fn increment_strong_count(ptr: *const T) {
        // Retain SRc, but don't touch refcount by wrapping in ManuallyDrop
        let rc = unsafe { mem::ManuallyDrop::new(SRc::<T>::from_raw(ptr)) };
        // Now increase refcount, but don't drop new refcount either
        let _rc_clone: mem::ManuallyDrop<_> = rc.clone();
    }

    /// Decrements the strong reference count on the `SRc<T>` associated with the
    /// provided pointer by one.
    ///
    /// # Safety
    ///
    /// The pointer must have been obtained through `SRc::into_raw`, and the
    /// associated `SRc` instance must be valid (i.e. the strong count must be at
    /// least 1) when invoking this method. This method can be used to release
    /// the final `SRc` and backing storage, but **should not** be called after
    /// the final `SRc` has been released.
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let five = SRc::new(5);
    ///
    /// unsafe {
    ///     let ptr = SRc::into_raw(five);
    ///     SRc::increment_strong_count(ptr);
    ///
    ///     let five = SRc::from_raw(ptr);
    ///     assert_eq!(2, SRc::strong_count(&five));
    ///     SRc::decrement_strong_count(ptr);
    ///     assert_eq!(1, SRc::strong_count(&five));
    /// }
    /// ```
    #[inline]
    #[stable(feature = "rc_mutate_strong_count", since = "1.53.0")]
    pub unsafe fn decrement_strong_count(ptr: *const T) {
        unsafe { mem::drop(SRc::from_raw(ptr)) };
    }

    /// Returns `true` if there are no other `SRc` or [`Weak`] pointers to
    /// this allocation.
    #[inline]
    fn is_unique(this: &Self) -> bool {
        SRc::weak_count(this) == 0 && SRc::strong_count(this) == 1
    }

    /// Returns a mutable reference into the given `SRc`, if there are
    /// no other `SRc` or [`Weak`] pointers to the same allocation.
    ///
    /// Returns [`None`] otherwise, because it is not safe to
    /// mutate a shared value.
    ///
    /// See also [`make_mut`][make_mut], which will [`clone`][clone]
    /// the inner value when there are other `SRc` pointers.
    ///
    /// [make_mut]: SRc::make_mut
    /// [clone]: Clone::clone
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let mut x = SRc::new(3);
    /// *SRc::get_mut(&mut x).unwrap() = 4;
    /// assert_eq!(*x, 4);
    ///
    /// let _y = SRc::clone(&x);
    /// assert!(SRc::get_mut(&mut x).is_none());
    /// ```
    #[inline]
    #[stable(feature = "rc_unique", since = "1.4.0")]
    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        if SRc::is_unique(this) { unsafe { Some(SRc::get_mut_unchecked(this)) } } else { None }
    }

    /// Returns a mutable reference into the given `SRc`,
    /// without any check.
    ///
    /// See also [`get_mut`], which is safe and does appropriate checks.
    ///
    /// [`get_mut`]: SRc::get_mut
    ///
    /// # Safety
    ///
    /// Any other `SRc` or [`Weak`] pointers to the same allocation must not be dereferenced
    /// for the duration of the returned borrow.
    /// This is trivially the case if no such pointers exist,
    /// for example immediately after `SRc::new`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(get_mut_unchecked)]
    ///
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let mut x = SRc::new(String::new());
    /// unsafe {
    ///     SRc::get_mut_unchecked(&mut x).push_str("foo")
    /// }
    /// assert_eq!(*x, "foo");
    /// ```
    #[inline]
    #[unstable(feature = "get_mut_unchecked", issue = "63292")]
    pub unsafe fn get_mut_unchecked(this: &mut Self) -> &mut T {
        // We are careful to *not* create a reference covering the "count" fields, as
        // this would conflict with accesses to the reference counts (e.g. by `Weak`).
        unsafe { &mut (*this.ptr.as_ptr()).value }
    }

    #[inline]
    #[stable(feature = "ptr_eq", since = "1.17.0")]
    /// Returns `true` if the two `SRc`s point to the same allocation
    /// (in a vein similar to [`ptr::eq`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let five = SRc::new(5);
    /// let same_five = SRc::clone(&five);
    /// let other_five = SRc::new(5);
    ///
    /// assert!(SRc::ptr_eq(&five, &same_five));
    /// assert!(!SRc::ptr_eq(&five, &other_five));
    /// ```
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        this.ptr.as_ptr() == other.ptr.as_ptr()
    }
}

impl<T: Clone> SRc<T> {
    /// Makes a mutable reference into the given `SRc`.
    ///
    /// If there are other `SRc` pointers to the same allocation, then `make_mut` will
    /// [`clone`] the inner value to a new allocation to ensure unique ownership.  This is also
    /// referred to as clone-on-write.
    ///
    /// However, if there are no other `SRc` pointers to this allocation, but some [`Weak`]
    /// pointers, then the [`Weak`] pointers will be disassociated and the inner value will not
    /// be cloned.
    ///
    /// See also [`get_mut`], which will fail rather than cloning the inner value
    /// or diassociating [`Weak`] pointers.
    ///
    /// [`clone`]: Clone::clone
    /// [`get_mut`]: SRc::get_mut
    ///
    /// # Examples
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let mut data = SRc::new(5);
    ///
    /// *SRc::make_mut(&mut data) += 1;         // Won't clone anything
    /// let mut other_data = SRc::clone(&data); // Won't clone inner data
    /// *SRc::make_mut(&mut data) += 1;         // Clones inner data
    /// *SRc::make_mut(&mut data) += 1;         // Won't clone anything
    /// *SRc::make_mut(&mut other_data) *= 2;   // Won't clone anything
    ///
    /// // Now `data` and `other_data` point to different allocations.
    /// assert_eq!(*data, 8);
    /// assert_eq!(*other_data, 12);
    /// ```
    ///
    /// [`Weak`] pointers will be disassociated:
    ///
    /// ```
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let mut data = SRc::new(75);
    /// let weak = SRc::downgrade(&data);
    ///
    /// assert!(75 == *data);
    /// assert!(75 == *weak.upgrade().unwrap());
    ///
    /// *SRc::make_mut(&mut data) += 1;
    ///
    /// assert!(76 == *data);
    /// assert!(weak.upgrade().is_none());
    /// ```
    #[cfg(not(no_global_oom_handling))]
    #[inline]
    #[stable(feature = "rc_unique", since = "1.4.0")]
    pub fn make_mut(this: &mut Self) -> &mut T {
        if SRc::strong_count(this) != 1 {
            // Gotta clone the data, there are other Rcs.
            // Pre-allocate memory to allow writing the cloned value directly.
            let mut rc = Self::new_uninit();
            unsafe {
                let data = SRc::get_mut_unchecked(&mut rc);
                (**this).write_clone_into_raw(data.as_mut_ptr());
                *this = rc.assume_init();
            }
        } else if SRc::weak_count(this) != 0 {
            // Can just steal the data, all that's left is Weaks
            let mut rc = Self::new_uninit();
            unsafe {
                let data = SRc::get_mut_unchecked(&mut rc);
                data.as_mut_ptr().copy_from_nonoverlapping(&**this, 1);

                this.inner().dec_strong();
                // Remove implicit strong-weak ref (no need to craft a fake
                // Weak here -- we know other Weaks can clean up for us)
                this.inner().dec_weak();
                ptr::write(this, rc.assume_init());
            }
        }
        // This unsafety is ok because we're guaranteed that the pointer
        // returned is the *only* pointer that will ever be returned to T. Our
        // reference count is guaranteed to be 1 at this point, and we required
        // the `SRc<T>` itself to be `mut`, so we're returning the only possible
        // reference to the allocation.
        unsafe { &mut this.ptr.as_mut().value }
    }

    /// If we have the only reference to `T` then unwrap it. Otherwise, clone `T` and return the
    /// clone.
    ///
    /// Assuming `rc_t` is of type `SRc<T>`, this function is functionally equivalent to
    /// `(*rc_t).clone()`, but will avoid cloning the inner value where possible.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(arc_unwrap_or_clone)]
    /// # use std::{ptr, rc::SRc};
    /// let inner = String::from("test");
    /// let ptr = inner.as_ptr();
    ///
    /// let rc = SRc::new(inner);
    /// let inner = SRc::unwrap_or_clone(rc);
    /// // The inner value was not cloned
    /// assert!(ptr::eq(ptr, inner.as_ptr()));
    ///
    /// let rc = SRc::new(inner);
    /// let rc2 = rc.clone();
    /// let inner = SRc::unwrap_or_clone(rc);
    /// // Because there were 2 references, we had to clone the inner value.
    /// assert!(!ptr::eq(ptr, inner.as_ptr()));
    /// // `rc2` is the last reference, so when we unwrap it we get back
    /// // the original `String`.
    /// let inner = SRc::unwrap_or_clone(rc2);
    /// assert!(ptr::eq(ptr, inner.as_ptr()));
    /// ```
    #[inline]
    #[unstable(feature = "arc_unwrap_or_clone", issue = "93610")]
    pub fn unwrap_or_clone(this: Self) -> T {
        SRc::try_unwrap(this).unwrap_or_else(|rc| (*rc).clone())
    }
}

impl SRc<dyn Any> {
    /// Attempt to downcast the `SRc<dyn Any>` to a concrete type.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::any::Any;
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// fn print_if_string(value: SRc<dyn Any>) {
    ///     if let Ok(string) = value.downcast::<String>() {
    ///         println!("String ({}): {}", string.len(), string);
    ///     }
    /// }
    ///
    /// let my_string = "Hello World".to_string();
    /// print_if_string(SRc::new(my_string));
    /// print_if_string(SRc::new(0i8));
    /// ```
    #[inline]
    #[stable(feature = "rc_downcast", since = "1.29.0")]
    pub fn downcast<T: Any>(self) -> Result<SRc<T>, SRc<dyn Any>> {
        if (*self).is::<T>() {
            unsafe {
                let ptr = self.ptr.cast::<SRcBox<T>>();
                forget(self);
                Ok(SRc::from_inner(ptr))
            }
        } else {
            Err(self)
        }
    }

    /// Downcasts the `SRc<dyn Any>` to a concrete type.
    ///
    /// For a safe alternative see [`downcast`].
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(downcast_unchecked)]
    ///
    /// use std::any::Any;
    /// use devolve_ui::data::scoped_rc::SRc;
    ///
    /// let x: SRc<dyn Any> = SRc::new(1_usize);
    ///
    /// unsafe {
    ///     assert_eq!(*x.downcast_unchecked::<usize>(), 1);
    /// }
    /// ```
    ///
    /// # Safety
    ///
    /// The contained value must be of type `T`. Calling this method
    /// with the incorrect type is *undefined behavior*.
    ///
    ///
    /// [`downcast`]: Self::downcast
    #[inline]
    #[unstable(feature = "downcast_unchecked", issue = "90850")]
    pub unsafe fn downcast_unchecked<T: Any>(self) -> SRc<T> {
        unsafe {
            let ptr = self.ptr.cast::<SRcBox<T>>();
            mem::forget(self);
            SRc::from_inner(ptr)
        }
    }
}

impl<T: ?Sized> SRc<T> {
    /// Allocates an `SRcBox<T>` with sufficient space for
    /// a possibly-unsized inner value where the value has the layout provided.
    ///
    /// The function `mem_to_rcbox` is called with the data pointer
    /// and must return back a (potentially fat)-pointer for the `SRcBox<T>`.
    #[cfg(not(no_global_oom_handling))]
    unsafe fn allocate_for_layout(
        value_layout: Layout,
        allocate: impl FnOnce(Layout) -> Result<NonNull<[u8]>, AllocError>,
        mem_to_rcbox: impl FnOnce(*mut u8) -> *mut SRcBox<T>,
    ) -> *mut SRcBox<T> {
        // Calculate layout using the given value layout.
        // Previously, layout was calculated on the expression
        // `&*(ptr as *const SRcBox<T>)`, but this created a misaligned
        // reference (see #54908).
        let layout = Layout::new::<SRcBox<()>>().extend(value_layout).unwrap().0.pad_to_align();
        unsafe {
            SRc::try_allocate_for_layout(value_layout, allocate, mem_to_rcbox)
                .unwrap_or_else(|_| handle_alloc_error(layout))
        }
    }

    /// Allocates an `SRcBox<T>` with sufficient space for
    /// a possibly-unsized inner value where the value has the layout provided,
    /// returning an error if allocation fails.
    ///
    /// The function `mem_to_rcbox` is called with the data pointer
    /// and must return back a (potentially fat)-pointer for the `SRcBox<T>`.
    #[inline]
    unsafe fn try_allocate_for_layout(
        value_layout: Layout,
        allocate: impl FnOnce(Layout) -> Result<NonNull<[u8]>, AllocError>,
        mem_to_rcbox: impl FnOnce(*mut u8) -> *mut SRcBox<T>,
    ) -> Result<*mut SRcBox<T>, AllocError> {
        // Calculate layout using the given value layout.
        // Previously, layout was calculated on the expression
        // `&*(ptr as *const SRcBox<T>)`, but this created a misaligned
        // reference (see #54908).
        let layout = Layout::new::<SRcBox<()>>().extend(value_layout).unwrap().0.pad_to_align();

        // Allocate for the layout.
        let ptr = allocate(layout)?;

        // Initialize the SRcBox
        let inner = mem_to_rcbox(ptr.as_non_null_ptr().as_ptr());
        unsafe {
            debug_assert_eq!(Layout::for_value(&*inner), layout);

            ptr::write(&mut (*inner).strong, Cell::new(1));
            ptr::write(&mut (*inner).weak, Cell::new(1));
        }

        Ok(inner)
    }

    /// Allocates an `SRcBox<T>` with sufficient space for an unsized inner value
    #[cfg(not(no_global_oom_handling))]
    unsafe fn allocate_for_ptr(ptr: *const T) -> *mut SRcBox<T> {
        // Allocate for the `SRcBox<T>` using the given value.
        unsafe {
            Self::allocate_for_layout(
                Layout::for_value(&*ptr),
                |layout| Global.allocate(layout),
                |mem| mem.with_metadata_of(ptr as *mut SRcBox<T>),
            )
        }
    }

    #[cfg(not(no_global_oom_handling))]
    fn from_box(v: Box<T>) -> SRc<T> {
        unsafe {
            let (box_unique, alloc) = Box::into_unique(v);
            let bptr = box_unique.as_ptr();

            let value_size = size_of_val(&*bptr);
            let ptr = Self::allocate_for_ptr(bptr);

            // Copy value as bytes
            ptr::copy_nonoverlapping(
                bptr as *const T as *const u8,
                &mut (*ptr).value as *mut _ as *mut u8,
                value_size,
            );

            // Free the allocation without dropping its contents
            box_free(box_unique, alloc);

            Self::from_ptr(ptr)
        }
    }
}
