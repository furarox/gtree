use crate::tree::{
    ChildIterator, ChildIteratorMut, ChildLink, LazyTreeIterator, LazyTreeIteratorMut, _iter_rec,
    _iter_rec_mut,
};
use std::{collections::LinkedList, marker::PhantomData};

/// Equivalent of immutable reference for [crate::Tree]
///
/// This structure owns a raw pointer, 'current', as in Tree, and can be used to navigate and peek
/// into the tree (same methods as in [crate::Tree]).
/// The main contribution of [Cursor] is that you can have multiple cursors together, meaning you
/// can make concurrent explorations of the tree.
///
/// Cursors are created by taking a reference to the tree, meaning that the borrow checker can
/// ensure it's normal behaviour as you would for normal references. As long as cursors are alive,
/// the tree cannot be mutated.
///
/// # Examples
/// ```
/// # use gtree::Tree;
/// let mut tree = Tree::from_element(10);
/// tree.push_iter(vec![1, 2, 3]);
/// tree.navigate_to(1);
///
/// // Multiples references we can move all along the tree
/// let cursor1 = tree.cursor();
/// let mut cursor2 = tree.cursor_root();
/// assert_eq!(cursor1.peek(), &2);
/// assert_eq!(cursor2.peek(), &10);
/// cursor2.navigate_to(2);
/// assert_eq!(cursor2.peek(), &3);
/// ```
pub struct Cursor<'a, T> {
    pub(crate) current: ChildLink<T>,
    pub(crate) _boo: PhantomData<&'a T>,
}

/// Equivalent of mutable reference for [crate::Tree]
///
/// This structure is the same as [Cursor], implements every methods that [Cursor] implements and
/// only implements a few more methods such as [CursorMut::peek_mut].
///
/// CursorMut are created by taking a mutable reference to the tree, meaning that they can be only
/// one CursorMut alive at the same time, and that creating a CursorMut invalidates every other
/// Cursor or CursorMut.
///
/// That behaviour makes CursorMut a bit useless, as it is the same as navigating 'current' in tree.
/// The most useful behaviour is when you need a mutable exploration of the tree without navigating
/// 'current', and so instead of navigating 'current' back to it's original location after you
/// exploration (which can be sometimes a bit complex), you can create a CursorMut, explores the
/// tree with it, and then throw it away as soon as you finished the exploration.
///
/// # Examples
/// ```
/// # use gtree::Tree;
/// let mut tree = Tree::from_element(10);
/// tree.push_iter(vec![1, 2, 3]);
/// // Firstly, node that we need a mut on the cursor definition.
/// // This is because the signature of peek_mut(&mut self) -> &mut T
/// // Doing so enforce rust ownership system, but it makes cursor immutable while the reference is
/// // alive.
/// let mut cursor = tree.cursor_mut();
/// let ref_el = cursor.peek_mut();
/// assert_eq!(ref_el, &mut 10);
/// *ref_el = 15;
/// cursor.navigate_to(0);
/// // but now we can't use ref_el, because navigate_to needs to mutate cursor, and so invalidates
/// // all mutable refences taken from cursor.
/// // Not very pratical if we, for exemple, want to build an iterator on the whole tree from a
/// // CursorMut.
/// ```
pub struct CursorMut<'a, T> {
    pub(crate) current: ChildLink<T>,
    pub(crate) _boo: PhantomData<&'a T>,
}

impl<'a, T> Cursor<'a, T> {
    /// Peek at 'current', returning a reference to the element stored in 'current'.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let tree = Tree::from_element(10);
    /// let cursor = tree.cursor();
    /// assert_eq!(cursor.peek(), &10);
    /// ```
    pub fn peek(&self) -> &'a T {
        unsafe { &(*self.current.as_ptr()).elem }
    }

    /// Peek at 'current'.childs\[index\], returning a reference to the element stored.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// tree.push(5);
    /// let cursor = tree.cursor();
    /// assert_eq!(cursor.peek_child(0), &5);
    /// ```
    pub fn peek_child(&self, index: usize) -> &'a T {
        if index >= self.childs_len() {
            panic!(
                "Tried to peek child on child {} but current has only {} childs",
                index,
                self.childs_len()
            );
        }

        unsafe { &(*(*self.current.as_ptr()).childs[index].as_ptr()).elem }
    }

    /// Set 'current' to 'current'.childs\[index\], therefore navigating to this child
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push(1);
    /// let mut cursor = tree.cursor();
    /// cursor.navigate_to(0);
    /// assert_eq!(cursor.peek(), &1);
    /// ```
    ///
    /// # Panics
    /// This method will panic if index >= self.childs_len
    pub fn navigate_to(&mut self, index: usize) {
        if index >= self.childs_len() {
            panic!(
                "Tried to navigate to child {} but current has only {} childs",
                index,
                self.childs_len()
            );
        }

        unsafe {
            self.current = (*self.current.as_ptr()).childs[index];
        }
    }

    /// Set 'current' to 'current'.father, therefore navigating up.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push(1);
    /// tree.navigate_to(0);
    /// let mut cursor = tree.cursor();
    /// cursor.ascend();
    /// assert_eq!(cursor.peek(), &0);
    /// ```
    ///
    /// # Panics
    /// This method will panic if 'current' has no father i.e. if 'current'.father.is_none()
    pub fn ascend(&mut self) {
        if !self.has_father() {
            panic!("Tried to call ascend but current has no father");
        }

        unsafe {
            self.current = (*self.current.as_ptr()).father.unwrap();
        }
    }

    /// Return true if 'current' has a father.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push(1);
    /// tree.navigate_to(0);
    /// let mut cursor = tree.cursor();
    /// assert!(cursor.has_father());
    /// cursor.ascend();
    /// assert!(!cursor.has_father());
    /// ```
    pub fn has_father(&self) -> bool {
        unsafe { (*self.current.as_ptr()).father.is_some() }
    }

    /// Return the number of childrens of current.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 1, 1, 1, 1]);
    /// let cursor = tree.cursor();
    /// assert_eq!(cursor.childs_len(), 5);
    /// ```
    pub fn childs_len(&self) -> usize {
        unsafe { (*self.current.as_ptr()).childs.len() }
    }

    /// Return an Iterator over the elements stored in 'current'.childs
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// let cursor = tree.cursor();
    /// assert_eq!(cursor.iter_childs().collect::<Vec<&i32>>(), vec![&1, &2, &3]);
    /// ```
    pub fn iter_childs(&self) -> ChildIterator<'a, T> {
        ChildIterator {
            current: self.current,
            i: 0,
            len: self.childs_len(),
            _boo: PhantomData,
        }
    }

    /// Iterate over references of element stored in the subtree rooted at 'current' in a
    /// depth-first way. This is done
    /// by creating a Vec and pushing every references into this Vec and then returning an iterator
    /// over this Vec. As it may not be very memory efficient, you might check [Cursor::lazyiter].
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// tree.navigate_to(1);
    /// tree.push(4);
    /// assert_eq!(tree.cursor().iter().collect::<Vec<&i32>>(), vec![&2, &4]);
    pub fn iter(&self) -> impl Iterator<Item = &'a T> {
        let mut container = Vec::new();
        _iter_rec(self.current, &mut container);
        container.into_iter()
    }

    /// Iterate over the subtree rooted at 'current' in a lazy depth-first way, returning
    /// references to the elements stored in the subtree. Although it is lazy iteration, meaning it is
    /// less stressfull for memory, it is slower than [Cursor::iter], because the cursor that is used
    /// to move around the tree has to keep tracks of which branches it has already explored.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// tree.navigate_to(1);
    /// tree.push_iter(vec![9, 8]);
    /// tree.ascend();
    /// tree.navigate_to(0);
    /// tree.push_iter(vec![9, 10]);
    /// tree.navigate_to(0);
    /// tree.push(15);
    /// tree.go_to_root();
    /// assert_eq!(
    ///     tree.lazyiter().collect::<Vec<&i32>>(),
    ///     vec![&0, &1, &9, &15, &10, &2, &9, &8, &3]
    /// );
    /// tree.navigate_to(1);
    /// assert_eq!(tree.lazyiter().collect::<Vec<&i32>>(), vec![&2, &9, &8]);
    /// ```
    pub fn lazyiter(&self) -> LazyTreeIterator<'a, T> {
        let mut idx_list = LinkedList::new();
        idx_list.push_back(0);
        LazyTreeIterator {
            cursor: Cursor {
                current: self.current,
                _boo: PhantomData,
            },
            idx_list,
            _boo: PhantomData,
        }
    }
}

impl<'a, T> CursorMut<'a, T> {
    /// Peek at 'current', returning a reference to the element stored in 'current'.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// let cursor = tree.cursor_mut();
    /// assert_eq!(cursor.peek(), &10);
    /// ```
    pub fn peek(&self) -> &'a T {
        unsafe { &(*self.current.as_ptr()).elem }
    }

    /// Peek at 'current', returning a mutable reference to the element stored in 'current'.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// let mut cursor = tree.cursor_mut();
    /// assert_eq!(cursor.peek_mut(), &mut 10);
    /// ```
    pub fn peek_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.current.as_ptr()).elem }
    }

    /// Peek at 'current'.childs\[index\], returning a reference to the element stored.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// tree.push(5);
    /// let cursor = tree.cursor_mut();
    /// assert_eq!(cursor.peek_child(0), &5);
    /// ```
    pub fn peek_child(&self, index: usize) -> &'a T {
        if index >= self.childs_len() {
            panic!(
                "Tried to peek child on child {} but current has only {} childs",
                index,
                self.childs_len()
            );
        }

        unsafe { &(*(*self.current.as_ptr()).childs[index].as_ptr()).elem }
    }

    /// Peek at 'current'.childs\[index\], returning a mutable reference to the element stored.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// tree.push(5);
    /// let mut cursor = tree.cursor_mut();
    /// assert_eq!(cursor.peek_child_mut(0), &mut 5);
    /// ```
    pub fn peek_child_mut(&mut self, index: usize) -> &mut T {
        if index >= self.childs_len() {
            panic!(
                "Tried to peek child on child {} but current has only {} childs",
                index,
                self.childs_len()
            );
        }

        unsafe { &mut (*(*self.current.as_ptr()).childs[index].as_ptr()).elem }
    }

    /// Set 'current' to 'current'.childs\[index\], therefore navigating to this child
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push(1);
    /// let mut cursor = tree.cursor_mut();
    /// cursor.navigate_to(0);
    /// assert_eq!(cursor.peek(), &1);
    /// ```
    ///
    /// # Panics
    /// This method will panic if index >= self.childs_len
    pub fn navigate_to(&mut self, index: usize) {
        if index >= self.childs_len() {
            panic!(
                "Tried to navigate to child {} but current has only {} childs",
                index,
                self.childs_len()
            );
        }

        unsafe {
            self.current = (*self.current.as_ptr()).childs[index];
        }
    }

    /// Set 'current' to 'current'.father, therefore navigating up.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push(1);
    /// tree.navigate_to(0);
    /// let mut cursor = tree.cursor_mut();
    /// cursor.ascend();
    /// assert_eq!(cursor.peek(), &0);
    /// ```
    ///
    /// # Panics
    /// This method will panic if 'current' has no father i.e. if 'current'.father.is_none()
    pub fn ascend(&mut self) {
        if !self.has_father() {
            panic!("Tried to call ascend but current has no father");
        }

        unsafe {
            self.current = (*self.current.as_ptr()).father.unwrap();
        }
    }

    /// Return true if 'current' has a father.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push(1);
    /// tree.navigate_to(0);
    /// let mut cursor = tree.cursor_mut();
    /// assert!(cursor.has_father());
    /// cursor.ascend();
    /// assert!(!cursor.has_father());
    /// ```
    pub fn has_father(&self) -> bool {
        unsafe { (*self.current.as_ptr()).father.is_some() }
    }

    /// Return the number of childrens of current.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 1, 1, 1, 1]);
    /// let cursor = tree.cursor_mut();
    /// assert_eq!(cursor.childs_len(), 5);
    /// ```
    pub fn childs_len(&self) -> usize {
        unsafe { (*self.current.as_ptr()).childs.len() }
    }

    /// Return an Iterator over the elements stored in 'current'.childs
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// let cursor = tree.cursor_mut();
    /// assert_eq!(cursor.iter_childs().collect::<Vec<&i32>>(), vec![&1, &2, &3]);
    /// ```
    pub fn iter_childs(&self) -> ChildIterator<'a, T> {
        ChildIterator {
            current: self.current,
            i: 0,
            len: self.childs_len(),
            _boo: PhantomData,
        }
    }

    /// Return an Iterator over the elements stored in 'current'.childs
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// let mut cursor = tree.cursor_mut();
    /// assert_eq!(cursor.iter_childs_mut().collect::<Vec<&mut i32>>(), vec![&mut 1, &mut 2, &mut 3]);
    /// ```
    pub fn iter_childs_mut(&self) -> ChildIteratorMut<'a, T> {
        ChildIteratorMut {
            current: self.current,
            i: 0,
            len: self.childs_len(),
            _boo: PhantomData,
        }
    }

    /// Iterate over references of element stored in the subtree rooted at 'current' in a
    /// depth-first way. This is done
    /// by creating a Vec and pushing every references into this Vec and then returning an iterator
    /// over this Vec. As it may not be very memory efficient, you might check [CursorMut::lazyiter].
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// tree.navigate_to(1);
    /// tree.push(4);
    /// assert_eq!(tree.cursor_mut().iter().collect::<Vec<&i32>>(), vec![&2, &4]);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &'a T> {
        let mut container = Vec::new();
        _iter_rec(self.current, &mut container);
        container.into_iter()
    }

    /// Same as [CursorMut::iter], but returns mutable reference instead
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// tree.navigate_to(1);
    /// tree.push(4);
    /// assert_eq!(tree.cursor_mut().iter_mut().collect::<Vec<&mut i32>>(), vec![&mut 2, &mut 4]);
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &'a mut T> {
        let mut container = Vec::new();
        _iter_rec_mut(self.current, &mut container);
        container.into_iter()
    }

    /// Iterate over the subtree rooted at 'current' in a lazy depth-first way, returning
    /// references to the elements stored in the subtree. Although it is lazy iteration, meaning it is
    /// less stressfull for memory, it is slower than [CursorMut::iter], because the cursor that is used
    /// to move around the tree has to keep tracks of which branches it has already explored.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// tree.navigate_to(1);
    /// tree.push_iter(vec![9, 8]);
    /// tree.ascend();
    /// tree.navigate_to(0);
    /// tree.push_iter(vec![9, 10]);
    /// tree.navigate_to(0);
    /// tree.push(15);
    /// tree.go_to_root();
    /// let mut cursor = tree.cursor_mut();
    /// assert_eq!(
    ///     cursor.lazyiter().collect::<Vec<&i32>>(),
    ///     vec![&0, &1, &9, &15, &10, &2, &9, &8, &3]
    /// );
    /// cursor.navigate_to(1);
    /// assert_eq!(cursor.lazyiter().collect::<Vec<&i32>>(), vec![&2, &9, &8]);
    /// ```
    pub fn lazyiter(&self) -> LazyTreeIterator<'a, T> {
        let mut idx_list = LinkedList::new();
        idx_list.push_back(0);
        LazyTreeIterator {
            cursor: Cursor {
                current: self.current,
                _boo: PhantomData,
            },
            idx_list,
            _boo: PhantomData,
        }
    }

    /// Same as [CursorMut::lazyiter] but returns mutable references instead
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// tree.navigate_to(1);
    /// tree.push_iter(vec![9, 8]);
    /// tree.ascend();
    /// tree.navigate_to(0);
    /// tree.push_iter(vec![9, 10]);
    /// tree.navigate_to(0);
    /// tree.push(15);
    /// tree.go_to_root();
    /// let mut cursor = tree.cursor_mut();
    /// assert_eq!(
    ///     cursor.lazyiter_mut().collect::<Vec<&mut i32>>(),
    ///     vec![&mut 0, &mut 1, &mut 9, &mut 15, &mut 10, &mut 2, &mut 9, &mut 8, &mut 3]
    /// );
    /// ```
    pub fn lazyiter_mut(&mut self) -> LazyTreeIteratorMut<'a, T> {
        let mut idx_list = LinkedList::new();
        idx_list.push_back(0);
        LazyTreeIteratorMut {
            cursor: UnsafeCursor {
                current: self.current,
                _boo: PhantomData,
            },
            idx_list,
            _boo: PhantomData,
        }
    }
}

/// An unsafe version of [CursorMut]
///
/// # Principle
/// UnsafeCursor exists to overcome the limitation of [CursorMut]. In order to respect the Rust
/// ownership system, mutable methods in CursorMut takes &mut self as signature. This means that
/// has long as the as the mutable reference is alive, it is not possible to mutate the cursor, to
/// navigate it around the tree, and to get a mutable reference to another node of the tree. If you
/// had to mutate the tree in multiple place in a concurrent way, it would be impossible with
/// CursorMut.
///
/// In order to achieve this, the narrator only sees two options :
/// - rewrite everything using RefCell and interior mutability (no.).
/// - implements a shotgun for the foot of the user of this crate (yes).
///
/// ## How do we achieve it
/// We simply change the signature of methods that return a mutable reference by
/// `fn peek_mut(&self) -> &mut T`.
/// Doing so disconnect the link that Rust was making between the &mut self and the &mut T
/// returned (as a immutable reference would never produce a mutable reference). This mean that we
/// can now navigate the UnsafeCursor while keeping the mutable reference alive, have multiples
/// mutable reference alive and even multiples UnsafeCursor alive. But as great powers came with
/// responsability, we also now can create two mutable references point at the same node
///
/// ```
/// # use gtree::Tree;
/// let mut tree = Tree::from_element(vec![10]);
/// let cursor1 = tree.unsafe_cursor();
/// let cursor2 = tree.unsafe_cursor();
/// let ref1 = unsafe { cursor1.peek_mut() };
/// let ref2 = unsafe { cursor2.peek_mut() };
/// // We now have two mutable references toward [10]
/// ```
///
/// # Safety
/// Using this unsafe cursor, we see that we have completly bypassed the rust ownership system, and
/// it will be a problem. What you should avoid at all costs is having two mutable references (or
/// more) that points at the same object.
///
/// But how to prevent this ? The first idea is to have only one UnsafeCursor that does a travel
/// around the tree but never peek_mut twice on the same node (This how [crate::Tree::lazyiter_mut] is implemented).
/// The second idea is to not keep the references, but the unsafe cursor and call peek_mut every
/// times you need to mutate the elements stored. Doing so will always deliver a correct mutable
/// reference, on the top of stack of pointer. This is why every mutable methods are marked as
/// unsafe.
///
/// UnsafeCursor are safe as long as you make sure to not have two mutable reference (or a
/// immutable reference followed by a mutable reference) on a same node of tree. The unsafe keyword
/// is more to warn you about the risk of using UnsafeCursor, but it is up to you to verify that
/// the use is safe.
///
/// To avoid shooting yourself in the foot, UnsafeCursor does not implement any iter methods,
/// except for children but only as immutable reference (to decide to which branch to navigate).
///
/// # A safe usage example
/// ```
/// # use gtree::Tree;
/// let mut tree = Tree::from_element(0);
/// tree.push_iter(vec![1, 2]);
/// let mut cursor1 = tree.unsafe_cursor();
/// let mut cursor2 = tree.unsafe_cursor();
/// cursor1.navigate_to(0);
/// cursor2.navigate_to(1);
/// unsafe {
/// assert_eq!(cursor1.peek_mut(), &mut 1);
/// assert_eq!(cursor2.peek_mut(), &mut 2);
/// }
/// ```
///
/// Anyways, if you don't need, don't use it.
pub struct UnsafeCursor<'a, T> {
    pub(crate) current: ChildLink<T>,
    pub(crate) _boo: PhantomData<&'a T>,
}

impl<'a, T> UnsafeCursor<'a, T> {
    /// Peek at 'current', returning a reference to the element stored in 'current'.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let tree = Tree::from_element(10);
    /// let cursor = tree.unsafe_cursor();
    /// assert_eq!(cursor.peek(), &10);
    /// ```
    pub fn peek(&self) -> &'a T {
        unsafe { &(*self.current.as_ptr()).elem }
    }

    /// Peek at 'current', returning a mutable reference to the element stored in 'current'.
    ///
    /// # Safety
    /// Bad usages of `peek_mut` can lead to two mutable references pointing at the same object. Be
    /// always sure when you use this method that no other mutable references or normal references
    /// are alive at the moment you use it.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let tree = Tree::from_element(1);
    /// let cursor = tree.unsafe_cursor();
    /// unsafe {assert_eq!(cursor.peek_mut(), &mut 1);}
    /// ```
    pub unsafe fn peek_mut(&self) -> &'a mut T {
        unsafe { &mut (*self.current.as_ptr()).elem }
    }

    /// Peek at 'current'.childs\[index\], returning a reference to the element stored.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// tree.push(5);
    /// let cursor = tree.unsafe_cursor();
    /// assert_eq!(cursor.peek_child(0), &5);
    /// ```
    pub fn peek_child(&self, index: usize) -> &'a T {
        if index >= self.childs_len() {
            panic!(
                "Tried to peek child on child {} but current has only {} childs",
                index,
                self.childs_len()
            );
        }

        unsafe { &(*(*self.current.as_ptr()).childs[index].as_ptr()).elem }
    }

    /// Peek at 'current'.childs\[index\], returning a mutable reference to the element stored in
    /// child.
    ///
    /// # Safety
    /// Bad usages of `peek_child_mut` can lead to two mutable references pointing at the same object. Be
    /// always sure when you use this method that no other mutable references or normal references
    /// are alive at the moment you use it.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// tree.push(5);
    /// let cursor = tree.unsafe_cursor();
    /// unsafe {assert_eq!(cursor.peek_child_mut(0), &5);}
    /// ```
    pub unsafe fn peek_child_mut(&self, index: usize) -> &'a mut T {
        if index >= self.childs_len() {
            panic!(
                "Tried to call peek_child_mut on child {} but current has only {} childs",
                index,
                self.childs_len()
            );
        }

        unsafe { &mut (*(*self.current.as_ptr()).childs[index].as_ptr()).elem }
    }

    /// Set 'current' to 'current'.childs\[index\], therefore navigating to this child
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push(1);
    /// let mut cursor = tree.unsafe_cursor();
    /// cursor.navigate_to(0);
    /// assert_eq!(cursor.peek(), &1);
    /// ```
    ///
    /// # Panics
    /// This method will panic if index >= self.childs_len
    pub fn navigate_to(&mut self, index: usize) {
        if index >= self.childs_len() {
            panic!(
                "Tried to navigate to child {} but current has only {} childs",
                index,
                self.childs_len()
            );
        }

        unsafe {
            self.current = (*self.current.as_ptr()).childs[index];
        }
    }

    /// Set 'current' to 'current'.father, therefore navigating up.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push(1);
    /// tree.navigate_to(0);
    /// let mut cursor = tree.unsafe_cursor();
    /// cursor.ascend();
    /// assert_eq!(cursor.peek(), &0);
    /// ```
    ///
    /// # Panics
    /// This method will panic if 'current' has no father i.e. if 'current'.father.is_none()
    pub fn ascend(&mut self) {
        if !self.has_father() {
            panic!("Tried to call ascend but current has no father");
        }

        unsafe {
            self.current = (*self.current.as_ptr()).father.unwrap();
        }
    }

    /// Return true if 'current' has a father.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push(1);
    /// tree.navigate_to(0);
    /// let mut cursor = tree.unsafe_cursor();
    /// assert!(cursor.has_father());
    /// cursor.ascend();
    /// assert!(!cursor.has_father());
    /// ```
    pub fn has_father(&self) -> bool {
        unsafe { (*self.current.as_ptr()).father.is_some() }
    }

    /// Return the number of childrens of current.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 1, 1, 1, 1]);
    /// let cursor = tree.unsafe_cursor();
    /// assert_eq!(cursor.childs_len(), 5);
    /// ```
    pub fn childs_len(&self) -> usize {
        unsafe { (*self.current.as_ptr()).childs.len() }
    }

    /// Return an Iterator over the elements stored in 'current'.childs
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// let cursor = tree.unsafe_cursor();
    /// assert_eq!(cursor.iter_childs().collect::<Vec<&i32>>(), vec![&1, &2, &3]);
    /// ```
    pub fn iter_childs(&self) -> ChildIterator<'a, T> {
        ChildIterator {
            current: self.current,
            i: 0,
            len: self.childs_len(),
            _boo: PhantomData,
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::Tree;

    #[test]
    fn unsafe_cursor1() {
        let mut tree = Tree::from_element(0);
        tree.push_iter(vec![1, 2, 3]);
        tree.navigate_to(1);
        tree.push_iter(vec![9, 8]);
        tree.ascend();
        tree.navigate_to(0);
        tree.push_iter(vec![9, 10]);
        tree.navigate_to(0);
        tree.push(15);
        tree.go_to_root();
        let mut cursor1 = tree.unsafe_cursor();
        cursor1.navigate_to(0);
        let mut cursor2 = tree.unsafe_cursor();
        cursor2.navigate_to(1);
        cursor2.navigate_to(1);
        unsafe { *cursor1.peek_mut() += 1 };
        unsafe { *cursor2.peek_mut() += 2 };
        assert_eq!(
            tree.lazyiter().collect::<Vec<&i32>>(),
            vec![&0, &2, &9, &15, &10, &2, &9, &10, &3]
        );
        unsafe { *cursor1.peek_mut() -= 10 };
        assert_eq!(
            tree.lazyiter().collect::<Vec<&i32>>(),
            vec![&0, &-8, &9, &15, &10, &2, &9, &10, &3]
        );
    }

    #[test]
    fn unsafe_cursor2() {
        let mut tree = Tree::from_element(vec![1, 2, 3]);
        tree.push_iter(vec![vec![4], vec![5, 6, 7, 8]]);
        let mut cursor1 = tree.unsafe_cursor();
        cursor1.navigate_to(0);
        let mut cursor2 = tree.unsafe_cursor();
        cursor2.navigate_to(1);
        unsafe { cursor2.peek_mut().push(9) };
        unsafe { cursor1.peek_mut().pop() };
        assert_eq!(
            tree.iter().collect::<Vec<&Vec<i32>>>(),
            vec![&vec![1, 2, 3], &vec![], &vec![5, 6, 7, 8, 9]]
        );
        unsafe {
            let vec = cursor2.peek_mut();
            while !vec.is_empty() {
                vec.pop();
            }
        }

        assert_eq!(
            tree.iter().collect::<Vec<&Vec<i32>>>(),
            vec![&vec![1, 2, 3], &vec![], &vec![]]
        );
    }
}
