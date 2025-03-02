use crate::{Cursor, CursorMut, UnsafeCursor};
use std::collections::LinkedList;
use std::convert::Into;
use std::marker::PhantomData;
use std::ptr::NonNull;

/// Represent a potential pointer to another Node
pub type Link<T> = Option<NonNull<Node<T>>>;

/// Represents a pointer to child node. Because childs pointers are stored in a [Vec], the null
/// case is handled by an empty Vec.
pub type ChildLink<T> = NonNull<Node<T>>;

/// Struture to represent a node in a tree
pub(crate) struct Node<T> {
    pub father: Link<T>,
    pub childs: Vec<ChildLink<T>>,
    pub elem: T,
}

/// The main structure in this tree crate
///
/// ## Data structure
/// A [Tree] object stores two raw pointers toward \[Node\](private struct) in the tree. The first one is the root,
/// and this the field that "owns" the data in the tree. What it means is that when Tree is
/// dropped, it is through the root.
/// The second field is another raw pointer, the 'current' pointer that allows to explore and
/// manipulate the tree. Every operation on the tree will always manipulate the 'current' pointer,
/// therefore we will describe the operation in terms of operation on 'current'.
///
/// If not written, every methods will panic if called on an empty tree. The main reason is that
/// most of them become ambigous if 'root' is None, therefore, except for very specific case, code
/// will panic.
/// Other possible panics are referenced in the documentation.
///
/// ## References
/// In order to have a concurrent exploration of the tree, this tree crate implements a special
/// type of cursor (as in [here](https://rust-unofficial.github.io/too-many-lists/fifth.html)).
/// This is due to the fact that in order to move around the tree, you need to change the 'current' pointer of the tree and therefore
/// invalidating every normal references to the tree. Check [Cursor], [CursorMut] and [UnsafeCursor]
/// for more detail.
pub struct Tree<T> {
    root: Link<T>,
    current: Link<T>,
    _boo: PhantomData<T>,
}

impl<T> Tree<T> {
    /// Creates a [Tree] from el. root and current will be pointing to the node holding el.
    pub fn from_element(el: T) -> Self {
        let node = unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                elem: el,
                childs: Vec::new(),
                father: None,
            })))
        };

        Tree {
            root: Some(node),
            current: Some(node),
            _boo: PhantomData,
        }
    }

    /// Return true if the tree is empty, i.e. if 'root' = None.
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// tree.push_iter(vec![1, 2, 3]);
    /// // empty the tree if called at root, check documentation for into_vec
    /// tree.into_vec();
    /// assert!(tree.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Push el to 'current'.child as a new node in the tree.
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(1);
    /// tree.push(2);
    /// tree.navigate_to(0);
    /// assert_eq!(tree.peek(), &2);
    /// ```
    pub fn push(&mut self, el: T) {
        if self.is_empty() {
            panic!("Tried to push an element to an empty tree")
        }
        unsafe {
            let current_node = &mut *(self.current.unwrap().as_ptr());
            current_node
                .childs
                .push(NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                    elem: el,
                    childs: Vec::new(),
                    father: self.current,
                }))))
        }
    }

    /// Convenient method to push the elements of an iterator into the tree.
    /// It's litteraly : for el in iter.into_iter() { tree.push(el) }
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// tree.navigate_to(2);
    /// assert_eq!(tree.peek(), &3);
    /// ```
    pub fn push_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for el in iter.into_iter() {
            self.push(el);
        }
    }

    /// Insert el into 'current'.childs at index.
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 3]);
    /// tree.insert(1, 2);
    /// tree.navigate_to(1);
    /// assert_eq!(tree.peek(), &2);
    /// ```
    pub fn insert(&mut self, index: usize, el: T) {
        if self.is_empty() {
            panic!("Tried to insert an element to an empty tree");
        }
        unsafe {
            let current_node = &mut *(self.current.unwrap().as_ptr());
            current_node.childs.insert(
                index,
                NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                    elem: el,
                    childs: Vec::new(),
                    father: self.current,
                }))),
            );
        }
    }

    /// Set current to 'current'.childs\[index\], therefore navigating current to it's idx childs.
    ///
    /// # Panic
    ///
    /// This method will panic if index > tree.childs_len()
    pub fn navigate_to(&mut self, index: usize) {
        if self.is_empty() {
            panic!("Tried to move to with an empty tree");
        }

        let current_node = unsafe { &*(self.current.unwrap().as_ptr()) };
        if index >= current_node.childs.len() {
            panic!(
                "Tried to move to children {} of current node, but current node has only {} childs",
                index,
                current_node.childs.len()
            );
        }
        self.current = Some(current_node.childs[index]);
    }

    /// Set current to 'current'.father, therefore naviguating current to it's father
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// tree.push(-10);
    /// tree.navigate_to(0);
    /// tree.ascend();
    /// assert_eq!(tree.peek(), &10);
    /// ```
    ///
    /// # Panics
    ///
    /// This method will panic if 'current' has no father, ie 'current'.father == None (woof woof
    /// python).
    pub fn ascend(&mut self) {
        if self.is_empty() {
            panic!("Tried to move up with an empty tree");
        }

        let current_node = unsafe { &(*self.current.unwrap().as_ptr()) };
        if current_node.father.is_none() {
            panic!("Tried to move up but current has no father");
        }
        self.current = current_node.father;
    }

    /// Return true if current has a father. Note that it will return false and not panic is tree
    /// is empty.
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// assert_eq!(tree.has_father(), false);
    /// tree.push(10);
    /// tree.navigate_to(0);
    /// assert_eq!(tree.has_father(), true);
    /// ```
    pub fn has_father(&self) -> bool {
        if self.is_empty() {
            return false;
        }
        unsafe { (*self.current.unwrap().as_ptr()).father.is_some() }
    }

    /// Set 'current' to 'tree.root', therefore navigating the tree back to root.
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(1);
    /// tree.push(2);
    /// tree.navigate_to(0);
    /// tree.push(3);
    /// tree.navigate_to(0);
    /// assert_eq!(tree.peek(), &3);
    /// tree.go_to_root();
    /// assert_eq!(tree.peek(), &1);
    /// ```
    pub fn go_to_root(&mut self) {
        if self.is_empty() {
            panic!("Tried to move to root on an empty tree");
        }
        self.current = self.root;
    }

    /// Peek at 'current', returning a reference to the element stored in 'current'
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let tree = Tree::from_element(32);
    /// assert_eq!(tree.peek(), &32);
    /// ```
    pub fn peek(&self) -> &T {
        if self.is_empty() {
            panic!("Tried to peek on an empty tree");
        }
        unsafe { &(*self.current.unwrap().as_ptr()).elem }
    }

    /// Same as [Tree::peek], but returns a mutable reference instead
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(3);
    /// assert_eq!(tree.peek_mut(), &mut 3);
    /// ```
    pub fn peek_mut(&mut self) -> &mut T {
        if self.is_empty() {
            panic!("Tried to peek mut on an empty tree");
        }
        unsafe { &mut (*self.current.unwrap().as_ptr()).elem }
    }

    /// Peek on 'current'.childs\[index\]
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// assert_eq!(tree.peek_child(2), &3);
    /// ```
    ///
    /// # Panics
    /// This method will panic if index >= tree.childs_len()
    pub fn peek_child(&self, index: usize) -> &T {
        if self.is_empty() {
            panic!("Tried to call peek_child on an empty tree");
        }

        if index >= self.childs_len() {
            panic!(
                "Tried to call peek_child on child {} but current has only {} childs",
                index,
                self.childs_len()
            );
        }

        unsafe { &(*(*self.current.unwrap().as_ptr()).childs[index].as_ptr()).elem }
    }

    /// Same as [Tree::peek_child] but returns a mutable reference.
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// assert_eq!(tree.peek_child_mut(2), &mut 3);
    /// ```
    ///
    /// # Panics
    /// This method will panic if index >= tree.childs_len()
    pub fn peek_child_mut(&mut self, index: usize) -> &mut T {
        if self.is_empty() {
            panic!("Tried to call peek_child_mut on an empty tree");
        }

        if index >= self.childs_len() {
            panic!(
                "Tried to call peek_child_mut on child {} but current has only {} childs",
                index,
                self.childs_len()
            );
        }

        unsafe { &mut (*(*self.current.unwrap().as_ptr()).childs[index].as_ptr()).elem }
    }
    /// Returns 'current'.childs.len
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// tree.push_iter(vec![1, 2, 3]);
    /// assert_eq!(tree.childs_len(), 3);
    /// ```
    pub fn childs_len(&self) -> usize {
        if self.is_empty() {
            panic!("Tried to call childs_len on an empty tree");
        }
        unsafe { (*self.current.unwrap().as_ptr()).childs.len() }
    }

    /// Return an iterator over the elements of current
    pub fn iter_childs(&self) -> ChildIterator<'_, T> {
        if self.is_empty() {
            panic!("Tried to call iter_childs on an empty tree");
        }
        ChildIterator {
            current: self.current.unwrap(),
            i: 0,
            len: self.childs_len(),
            _boo: PhantomData,
        }
    }

    /// Return a mutuable iterator over the elements of current
    pub fn iter_childs_mut(&self) -> ChildIteratorMut<'_, T> {
        if self.is_empty() {
            panic!("Tried to call iter_childs on an empty tree");
        }
        ChildIteratorMut {
            current: self.current.unwrap(),
            i: 0,
            len: self.childs_len(),
            _boo: PhantomData,
        }
    }

    /// Insert the other tree into 'current'.childs at index
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree1 = Tree::from_element(0);
    /// tree1.push_iter(vec![1, 2, 3]);
    /// let mut tree2 = Tree::from_element(4);
    /// tree2.push_iter(vec![5, 6]);
    /// tree1.join(tree2, 1);
    /// tree1.navigate_to(1);
    /// assert_eq!(tree1.iter_childs().collect::<Vec<&i32>>(), vec![&5, &6]);
    /// ```
    ///
    /// # Panics
    /// This method panic if either of the trees are empty
    pub fn join(&mut self, mut other: Tree<T>, index: usize) {
        if self.is_empty() || other.root.is_none() {
            panic!("Tried to call join on an empty tree");
        }

        let other_root = other.root.unwrap();
        // Very important, otherwise, when other get dropped, it will dropped it's old data in
        // current tree
        other.root = None;
        unsafe {
            (*other_root.as_ptr()).father = self.current;
            (*self.current.unwrap().as_ptr())
                .childs
                .insert(index, other_root);
        }
    }

    /// Remove from 'current' the subtree rooted in 'current'.childs\[index\] and return it as a new
    /// tree. This method also serves a remove method. It can also be used to dropped the subtree
    /// above the node you want to split at. It really does a lot of things...
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2]);
    /// tree.navigate_to(1);
    /// tree.push_iter(vec![3, 4]);
    /// tree.ascend();
    /// let split_tree = tree.split(1);
    /// assert_eq!(split_tree.peek(), &2);
    /// assert_eq!(
    ///     split_tree.iter_childs().collect::<Vec<&i32>>(),
    ///     vec![&3, &4]
    /// );
    /// assert_eq!(split_tree.has_father(), false);
    /// ```
    ///
    /// # Panics
    /// This method will panic if index >= tree.childs_len()
    pub fn split(&mut self, index: usize) -> Tree<T> {
        if self.is_empty() {
            panic!("Tried to call split on an empty tree");
        }

        let current = self.current.unwrap();
        unsafe {
            if index >= self.childs_len() {
                panic!(
                    "Tried to call split with index {} but current has only {} childs",
                    index,
                    self.childs_len()
                );
            }

            let split_node = (*current.as_ptr()).childs.remove(index);
            (*split_node.as_ptr()).father = None;
            Tree {
                root: Some(split_node),
                current: Some(split_node),
                _boo: PhantomData,
            }
        }
    }

    /// Return a [Cursor] pointing at 'current'
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let tree = Tree::from_element(5);
    /// let cursor = tree.cursor();
    /// assert_eq!(cursor.peek(), &5);
    /// ```
    pub fn cursor(&self) -> Cursor<'_, T> {
        if self.is_empty() {
            panic!("Tried to call cursor on an empty tree");
        }

        Cursor {
            current: self.current.unwrap(),
            _boo: PhantomData,
        }
    }

    /// Return a [CursorMut] pointing at 'current'
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(5);
    /// let mut cursor = tree.cursor_mut();
    /// assert_eq!(cursor.peek_mut(), &mut 5);
    /// ```
    pub fn cursor_mut(&mut self) -> CursorMut<'_, T> {
        if self.is_empty() {
            panic!("Tried to call cursor on an empty tree");
        }

        CursorMut {
            current: self.current.unwrap(),
            _boo: PhantomData,
        }
    }

    /// Return an [UnsafeCursor] pointing at 'current'
    ///
    /// # Safety
    /// Creating an UnsafeCursor is not unsafe in itself, but its utilisation can lead to unsafe
    /// behaviour. Please read carefully [UnsafeCursor] documentation.
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let tree = Tree::from_element(5);
    /// let mut cursor = tree.unsafe_cursor();
    /// unsafe {assert_eq!(cursor.peek_mut(), &mut 5)};
    /// ```
    pub fn unsafe_cursor(&self) -> UnsafeCursor<'_, T> {
        if self.is_empty() {
            panic!("Tried to call cursor on an empty tree");
        }

        UnsafeCursor {
            current: self.current.unwrap(),
            _boo: PhantomData,
        }
    }

    /// Return a [Cursor] pointing at 'root'
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(3);
    /// tree.push(10);
    /// tree.navigate_to(0);
    /// let cursor = tree.cursor_root();
    /// assert_eq!(cursor.peek(), &3);
    /// ```
    pub fn cursor_root(&self) -> Cursor<'_, T> {
        if self.is_empty() {
            panic!("Tried to call cursor_root on an empty tree");
        }

        Cursor {
            current: self.root.unwrap(),
            _boo: PhantomData,
        }
    }

    /// Return a [CursorMut] pointing at 'root'
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(3);
    /// tree.push(10);
    /// tree.navigate_to(0);
    /// let mut cursor = tree.cursor_root_mut();
    /// assert_eq!(cursor.peek(), &3);
    /// ```
    pub fn cursor_root_mut(&mut self) -> CursorMut<'_, T> {
        if self.is_empty() {
            panic!("Tried to call cursor_root on an empty tree");
        }

        CursorMut {
            current: self.root.unwrap(),
            _boo: PhantomData,
        }
    }

    /// Return an [UnsafeCursor] pointing at 'root'
    ///
    /// # Safety
    /// Creating an UnsafeCursor is not unsafe in itself, but its utilisation can lead to unsafe
    /// behaviour. Please read carefully [UnsafeCursor] documentation.
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(5);
    /// tree.push(10);
    /// tree.navigate_to(0);
    /// let mut cursor = tree.unsafe_cursor_root();
    /// unsafe {assert_eq!(cursor.peek_mut(), &mut 5)};
    /// ```
    pub fn unsafe_cursor_root(&self) -> UnsafeCursor<'_, T> {
        if self.is_empty() {
            panic!("Tried to call cursor on an empty tree");
        }

        UnsafeCursor {
            current: self.root.unwrap(),
            _boo: PhantomData,
        }
    }

    /// Collect the subtree in a depth-first order rooted at 'current' into a vec, and ascend 'current'.
    /// If 'current' is at 'root', and so it cannot ascend, the tree becomes an empty tree (and so
    /// most method will therefore fail).
    /// This method is the main way to collect back elements stored in the tree.
    ///
    /// This behaviour is different from `Into<Vec>` implemented, where the whole tree is turned into
    /// a Vec, and not just a subtree.
    ///
    /// # Examples
    /// ```
    /// // Example on a subtree
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// tree.navigate_to(0);
    /// tree.push_iter(vec![9, 10]);
    /// tree.navigate_to(0);
    /// tree.push(15);
    /// tree.go_to_root();
    /// tree.navigate_to(0);
    /// assert_eq!(tree.into_vec(), vec![1, 9, 15, 10]);
    /// // 'current' has ascend and is now at 'root'
    /// assert_eq!(tree.peek(), &0);
    /// ```
    /// ```
    /// // Example at 'root'
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element([0, 0]);
    /// tree.push_iter([[1, 2], [3, 4]]);
    /// assert_eq!(tree.into_vec(), vec![[0, 0], [1, 2], [3, 4]]);
    /// // Calling tree.peek() will now panic because tree has become empty
    /// ```
    pub fn into_vec(&mut self) -> Vec<T> {
        if self.is_empty() {
            panic!("Tried to call into_vec on an empty tree");
        }

        if !self.has_father() {
            // we are at root
            let mut container = Vec::new();
            _into_vec_rec(self.root.unwrap(), &mut container);
            // Clean pointer to avoid so that the tree drop won't cause double free
            self.root = None;
            self.current = None;
            container
        } else {
            // we are not a root, so we ascend and we split the branch that is to be turned into a
            // vec
            let mut container = Vec::new();
            let old_current = self.current.unwrap();
            self.ascend();
            unsafe {
                for (idx, child) in (*self.current.unwrap().as_ptr()).childs.iter().enumerate() {
                    if *child == old_current {
                        let mut old_tree = self.split(idx);
                        _into_vec_rec(old_current, &mut container);
                        // Clean pointer to avoid so that the tree drop won't cause double free
                        old_tree.root = None;
                        old_tree.current = None;
                        break;
                    }
                }
            }
            container
        }
    }

    /// Iterate over references of element stored in the subtree rooted at 'current' in a
    /// depth-first way. This is done
    /// by creating a Vec and pushing every references into this Vec and then returning an iterator
    /// over this Vec. As it may not be very memory efficient, you might check [Tree::lazyiter].
    /// Also note that this method will not panic if called on an empty tree.
    ///
    /// Because the behaviour of iter is not very explicit, [Tree] does not implement the Iterator
    /// trait.
    ///
    /// If you need to iter over the whole tree but without navigating 'current', you can use a
    /// [Cursor] and send him to root and then call [Cursor::iter].
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// tree.navigate_to(1);
    /// tree.push(4);
    /// tree.ascend();
    /// assert_eq!(tree.iter().collect::<Vec<&i32>>(), vec![&0, &1, &2, &4, &3]);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        if self.is_empty() {
            return Vec::new().into_iter();
        }
        let mut container = Vec::new();
        _iter_rec(self.current.unwrap(), &mut container);
        container.into_iter()
    }

    /// Iterate over mutable references of element stored in the subtree rooted at 'current' in a
    /// depth-first way. This is done
    /// by creating a Vec and pushing every references into this Vec and then returning an iterator
    /// over this Vec. As it may not be very memory efficient, you might check [Tree::lazyiter].
    /// Also note that this method will not panic if called on an empty tree.
    ///
    /// If you need to iter over the whole tree but without navigating 'current', you can use a
    /// [CursorMut] and send him to root and then call [CursorMut::iter].
    /// # Examples
    /// ```
    /// # use libtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// tree.navigate_to(1);
    /// tree.push(4);
    /// tree.ascend();
    /// assert_eq!(tree.iter_mut().collect::<Vec<&mut i32>>(), vec![&mut 0, &mut 1, &mut 2, &mut 4, &mut 3]);
    /// ```
    pub fn iter_mut(&self) -> impl Iterator<Item = &mut T> {
        if self.is_empty() {
            return Vec::new().into_iter();
        }
        let mut container = Vec::new();
        _iter_rec_mut(self.current.unwrap(), &mut container);
        container.into_iter()
    }

    /// Iterate over the subtree rooted at 'current' in a lazy depth-first way, returning
    /// references to the elements stored in the subtree. Although it is lazy iteration, meaning it is
    /// less stressfull for memory, it is slower than [Tree::iter], because the cursor that is used
    /// to move around the tree has to keep tracks of which branches it has already explored.
    ///
    /// # Examples
    /// ```
    /// # use libtree::Tree;
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
    pub fn lazyiter(&self) -> LazyTreeIterator<'_, T> {
        if self.is_empty() {
            panic!("Tried to call lazyiter on an empty tree");
        }
        let mut idx_vec = LinkedList::new();
        idx_vec.push_back(0);
        LazyTreeIterator {
            cursor: self.cursor(),
            idx_list: idx_vec,
            _boo: PhantomData,
        }
    }

    pub fn lazyiter_mut(&mut self) -> LazyTreeIteratorMut<'_, T> {
        if self.is_empty() {
            panic!("Tried to call lazyiter_mut on an empty tree");
        }

        let mut idx_vec = LinkedList::new();
        idx_vec.push_back(0);
        LazyTreeIteratorMut {
            cursor: self.unsafe_cursor(),
            idx_list: idx_vec,
            _boo: PhantomData,
        }
    }
}

pub struct ChildIterator<'a, T> {
    pub(crate) current: ChildLink<T>,
    pub(crate) i: usize,
    pub(crate) _boo: PhantomData<&'a T>,
    pub(crate) len: usize,
}

impl<'a, T> Iterator for ChildIterator<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.len {
            let item = unsafe { &(*(*self.current.as_ptr()).childs[self.i].as_ptr()).elem };
            self.i += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub struct ChildIteratorMut<'a, T> {
    pub(crate) current: ChildLink<T>,
    pub(crate) i: usize,
    pub(crate) _boo: PhantomData<&'a T>,
    pub(crate) len: usize,
}

impl<'a, T> Iterator for ChildIteratorMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.len {
            let item = unsafe { &mut (*(*self.current.as_ptr()).childs[self.i].as_ptr()).elem };
            self.i += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub struct LazyTreeIterator<'a, T> {
    pub(crate) cursor: Cursor<'a, T>,
    pub(crate) idx_list: LinkedList<usize>,
    pub(crate) _boo: PhantomData<&'a T>,
}

impl<'a, T> Iterator for LazyTreeIterator<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx_list.is_empty() {
            return None;
        }

        let res;
        if self.cursor.childs_len() == 0 {
            res = Some(self.cursor.peek());
            self.cursor.ascend();
            self.idx_list.pop_back();
        } else if *self.idx_list.back().unwrap() < self.cursor.childs_len() {
            if *self.idx_list.back().unwrap() == 0 {
                res = Some(self.cursor.peek());
                self.cursor.navigate_to(0);
                *self.idx_list.back_mut().unwrap() += 1;
                self.idx_list.push_back(0);
            } else {
                self.cursor.navigate_to(*self.idx_list.back().unwrap());
                *self.idx_list.back_mut().unwrap() += 1;
                self.idx_list.push_back(0);
                res = self.next();
            }
        } else {
            self.idx_list.pop_back();
            if self.cursor.has_father() {
                self.cursor.ascend();
            }
            res = self.next();
        }

        res
    }
}

pub struct LazyTreeIteratorMut<'a, T> {
    pub(crate) cursor: UnsafeCursor<'a, T>,
    pub(crate) idx_list: LinkedList<usize>,
    pub(crate) _boo: PhantomData<&'a T>,
}

impl<'a, T> Iterator for LazyTreeIteratorMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx_list.is_empty() {
            return None;
        }

        let res;
        if self.cursor.childs_len() == 0 {
            unsafe {
                res = Some(self.cursor.peek_mut());
            }
            self.cursor.ascend();
            self.idx_list.pop_back();
        } else if *self.idx_list.back().unwrap() < self.cursor.childs_len() {
            if *self.idx_list.back().unwrap() == 0 {
                unsafe {
                    res = Some(self.cursor.peek_mut());
                }
                self.cursor.navigate_to(0);
                *self.idx_list.back_mut().unwrap() += 1;
                self.idx_list.push_back(0);
            } else {
                self.cursor.navigate_to(*self.idx_list.back().unwrap());
                *self.idx_list.back_mut().unwrap() += 1;
                self.idx_list.push_back(0);
                res = self.next();
            }
        } else {
            self.idx_list.pop_back();
            if self.cursor.has_father() {
                self.cursor.ascend();
            }
            res = self.next();
        }

        res
    }
}
/// Recursive function to gather reference of the subtree into container.
pub fn _iter_rec<T>(link: ChildLink<T>, container: &mut Vec<&T>) {
    unsafe {
        container.push(&(*link.as_ptr()).elem);
        for child in (*link.as_ptr()).childs.iter() {
            _iter_rec(*child, container);
        }
    }
}

/// Recursive function to gather mutable reference of the subtree into container.
pub fn _iter_rec_mut<T>(link: ChildLink<T>, container: &mut Vec<&mut T>) {
    unsafe {
        container.push(&mut (*link.as_ptr()).elem);
        for child in (*link.as_ptr()).childs.iter() {
            _iter_rec_mut(*child, container);
        }
    }
}

/// Reursive function to turn a subtree into a vec.
fn _into_vec_rec<T>(link_node: ChildLink<T>, container: &mut Vec<T>) {
    unsafe {
        let boxed_node = Box::from_raw(link_node.as_ptr());
        let (el, childs) = (boxed_node.elem, boxed_node.childs);
        container.push(el);

        for child in childs {
            _into_vec_rec(child, container);
        }
    }
}

/// Recursive function to clone the tree under cursor.
fn _clone_rec<T>(
    cursor: &mut Cursor<'_, T>,
    new_tree: &mut Tree<T>,
    tree: &Tree<T>,
) -> Option<NonNull<Node<T>>>
where
    T: Clone,
{
    let mut res = None;
    if cursor.current == tree.current.unwrap() {
        res = new_tree.current;
    }

    if cursor.childs_len() > 0 {
        for i in 0..cursor.childs_len() {
            new_tree.push(cursor.peek_child(i).clone());
            new_tree.navigate_to(i);
            cursor.navigate_to(i);
            let ret = _clone_rec(cursor, new_tree, tree);
            if ret.is_some() {
                res = ret;
            }
            new_tree.ascend();
            cursor.ascend();
        }
    }

    res
}

impl<T> Default for Tree<T> {
    fn default() -> Self {
        Tree {
            current: None,
            root: None,
            _boo: PhantomData,
        }
    }
}

impl<T> Clone for Tree<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        if self.is_empty() {
            panic!("Tried to call clone on an empty tree");
        }

        let mut cursor = self.cursor_root();
        let mut new_tree = Self::from_element(cursor.peek().clone());
        let new_current = _clone_rec(&mut cursor, &mut new_tree, self);
        new_tree.current = new_current;
        new_tree
    }
}

impl<T> Into<Vec<T>> for Tree<T> {
    fn into(mut self) -> Vec<T> {
        self.go_to_root();
        self.into_vec()
    }
}

impl<T> Drop for Tree<T> {
    fn drop(&mut self) {
        if self.root.is_some() {
            self.go_to_root();

            for _ in 0..self.childs_len() {
                self.split(0);
            }
            unsafe {
                let _ = Box::from_raw(self.current.unwrap().as_ptr());
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn peek() {
        let mut tree = Tree::from_element(4);
        assert_eq!(tree.peek(), &4);
        assert_eq!(tree.peek_mut(), &mut 4);
    }

    #[test]
    fn move_to_no_panic() {
        let mut tree = Tree::from_element(3);
        tree.push(5);
        tree.push(-1);
        tree.navigate_to(1);
        assert_eq!(tree.peek(), &-1);
    }

    #[test]
    #[should_panic(
        expected = "Tried to move to children 1 of current node, but current node has only 1 childs"
    )]
    fn move_to_panic() {
        let mut tree = Tree::from_element(3);
        tree.push(3);
        tree.navigate_to(1);
    }

    #[test]
    fn move_up() {
        let mut tree = Tree::from_element(20);
        tree.push(1);
        tree.navigate_to(0);
        tree.ascend();
        assert_eq!(tree.peek(), &20);
    }

    #[test]
    fn move_to_root() {
        let mut tree = Tree::from_element(10);
        tree.push(1);
        tree.navigate_to(0);
        tree.push(15);
        tree.navigate_to(0);
        tree.go_to_root();
        assert_eq!(tree.peek(), &10);
    }

    #[test]
    fn push() {
        let mut tree = Tree::from_element(10);
        tree.push(2);
        tree.push(3);
        tree.push(4);
        let vec: Vec<&i32> = tree.iter_childs().collect();
        assert_eq!(vec, vec![&2, &3, &4]);
    }

    #[test]
    fn insert() {
        let mut tree = Tree::from_element(10);
        tree.push(1);
        tree.push(3);
        tree.insert(1, 2);
        assert_eq!(tree.iter_childs().collect::<Vec<&i32>>(), vec![&1, &2, &3]);
    }

    #[test]
    fn iter_mutchilds() {
        let mut tree = Tree::from_element(10);
        tree.push_iter(vec![1, 2, 3]);
        assert_eq!(
            tree.iter_childs_mut().collect::<Vec<&mut i32>>(),
            vec![&mut 1, &mut 2, &mut 3]
        );
    }

    #[test]
    fn join() {
        let mut tree1 = Tree::from_element(0);
        let mut tree2 = Tree::from_element(1);
        tree2.push_iter(vec![2, 3]);
        tree1.join(tree2, 0);
        tree1.navigate_to(0);
        assert_eq!(tree1.peek(), &1);
        assert_eq!(tree1.iter_childs().collect::<Vec<&i32>>(), vec![&2, &3]);
    }

    #[test]
    fn split() {
        let mut tree = Tree::from_element(0);
        tree.push_iter(vec![1, 2]);
        tree.navigate_to(1);
        tree.push_iter(vec![3, 4]);
        tree.ascend();
        let split_tree = tree.split(1);
        assert!(!split_tree.has_father());
        assert_eq!(split_tree.peek(), &2);
        assert_eq!(
            split_tree.iter_childs().collect::<Vec<&i32>>(),
            vec![&3, &4]
        );

        std::mem::drop(tree);
        assert_eq!(split_tree.peek(), &2);
    }

    #[test]
    #[should_panic(expected = "Tried to call split with index 3 but current has only 0 childs")]
    fn split_panic() {
        let mut tree = Tree::from_element(0);
        tree.split(3);
    }

    #[test]
    #[should_panic(
        expected = "Tried to move to children 1 of current node, but current node has only 1 childs"
    )]
    fn split_no_dangling_pointer() {
        let mut tree = Tree::from_element(0);
        tree.push_iter(vec![1, 2]);
        let _ = tree.split(1);
        tree.navigate_to(1);
    }

    #[test]
    fn clone() {
        let mut tree = Tree::from_element(0);
        tree.push_iter(vec![1, 2, 3]);
        tree.navigate_to(0);
        tree.push_iter(vec![4, 5]);
        tree.ascend();
        tree.navigate_to(1);
        tree.push_iter(vec![6, 7, 8]);
        tree.ascend();
        tree.navigate_to(2);
        tree.push(9);
        tree.navigate_to(0);
        tree.push_iter(vec![1, 2, 3]);

        let mut clone = tree.clone();
        std::mem::drop(tree);

        assert_eq!(clone.peek(), &9);
        assert_eq!(clone.iter_childs().collect::<Vec<&i32>>(), vec![&1, &2, &3]);
        clone.ascend();
        clone.ascend();
        clone.navigate_to(1);
        assert_eq!(clone.iter_childs().collect::<Vec<&i32>>(), vec![&6, &7, &8]);
        clone.go_to_root();
        assert_eq!(clone.peek(), &0);
        assert_eq!(clone.iter_childs().collect::<Vec<&i32>>(), vec![&1, &2, &3]);
    }

    #[test]
    fn memory_leak() {
        let mut tree = Tree::from_element(vec![1, 2, 3]);
        tree.push(vec![4, 5, 6]);
    }

    #[test]
    fn into_vec() {
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
        tree.navigate_to(0);
        let vec = tree.into_vec();
        assert_eq!(vec, vec![1, 9, 15, 10]);
        assert_eq!(tree.peek(), &0);
        tree.navigate_to(0);
        assert_eq!(tree.peek(), &2);
        tree.navigate_to(1);
        assert_eq!(tree.peek(), &8)
    }

    #[test]
    #[should_panic(expected = "Tried to peek on an empty tree")]
    fn into_vec_root() {
        let mut tree = Tree::from_element([0, 0]);
        tree.push_iter([[1, 2], [3, 4]]);
        assert_eq!(tree.into_vec(), vec![[0, 0], [1, 2], [3, 4]]);
        tree.peek();
    }

    #[test]
    fn lazyiter() {
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
        assert_eq!(
            tree.lazyiter().collect::<Vec<&i32>>(),
            vec![&0, &1, &9, &15, &10, &2, &9, &8, &3]
        );
        tree.navigate_to(1);
        assert_eq!(tree.lazyiter().collect::<Vec<&i32>>(), vec![&2, &9, &8]);
    }

    #[test]
    fn lazyiter_mut() {
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
        assert_eq!(
            tree.lazyiter_mut().collect::<Vec<&mut i32>>(),
            vec![&mut 0, &mut 1, &mut 9, &mut 15, &mut 10, &mut 2, &mut 9, &mut 8, &mut 3]
        );

        for el in tree.lazyiter_mut() {
            *el += 1
        }
        assert_eq!(
            tree.lazyiter().collect::<Vec<&i32>>(),
            vec![&1, &2, &10, &16, &11, &3, &10, &9, &4]
        );

        tree.navigate_to(1);
        for el in tree.lazyiter_mut() {
            *el += 10;
        }
        tree.go_to_root();

        assert_eq!(tree.into_vec(), [1, 2, 10, 16, 11, 13, 20, 19, 4]);
    }
}
