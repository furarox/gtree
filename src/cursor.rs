use crate::tree::{ChildIterator, ChildLink, LazyTreeIterator, _iter_rec};
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
    pub fn peek_child(&self, index: usize) -> &T {
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
    pub fn iter_childs(&self) -> ChildIterator<'_, T> {
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
    pub fn iter(&self) -> impl Iterator<Item = &T> {
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
