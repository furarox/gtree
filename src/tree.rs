use crate::cursor::Cursor;
use std::marker::PhantomData;
use std::ptr::NonNull;

/// Represent a potential pointer to another Node
pub type Link<T> = Option<NonNull<Node<T>>>;

/// Represents a pointer to child node. Because childs pointers are stored in a [Vec], the null
/// case is handled by an empty Vec.
pub type ChildLink<T> = NonNull<Node<T>>;

/// Struture to represent a node in a tree
pub struct Node<T> {
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
/// most of them become ambigous if root is None, therefore, except for very specific case, code
/// will panic.
/// Other possible panics are referenced in the documentation.
///
/// ## References
/// In order to have a concurrent exploration of the tree, this tree crate implements a special
/// type of cursor (as in [here](https://rust-unofficial.github.io/too-many-lists/fifth.html)).
/// Because in order to move around the tree, you need to change the pointer of the tree and so it
/// will invalidate every normal references to the tree. Check [Cursor], [CursorMut] and
/// [UnsafeCursor] (use it at your own risks) for more detail.
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

    /// Push el to 'current'.child as a new node in the tree.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(1);
    /// tree.push(2);
    /// tree.navigate_to(0);
    /// assert_eq!(tree.peek(), &2);
    /// ```
    pub fn push(&mut self, el: T) {
        if self.root.is_none() {
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
    /// # use gtree::Tree;
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
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 3]);
    /// tree.insert(1, 2);
    /// tree.navigate_to(1);
    /// assert_eq!(tree.peek(), &2);
    /// ```
    pub fn insert(&mut self, index: usize, el: T) {
        if self.root.is_none() {
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
        if self.root.is_none() {
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
    /// # use gtree::Tree;
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
        if self.root.is_none() {
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
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// assert_eq!(tree.has_father(), false);
    /// tree.push(10);
    /// tree.navigate_to(0);
    /// assert_eq!(tree.has_father(), true);
    /// ```
    pub fn has_father(&self) -> bool {
        if self.root.is_none() {
            return false;
        }
        unsafe { (*self.current.unwrap().as_ptr()).father.is_some() }
    }

    /// Set 'current' to 'tree.root', therefore navigating the tree back to root.
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
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
        if self.root.is_none() {
            panic!("Tried to move to root on an empty tree");
        }
        self.current = self.root;
    }

    /// Peek at 'current', returning a reference to the element stored in 'current'
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let tree = Tree::from_element(32);
    /// assert_eq!(tree.peek(), &32);
    /// ```
    pub fn peek(&self) -> &T {
        if self.root.is_none() {
            panic!("Tried to peek on an empty tree");
        }
        unsafe { &(*self.current.unwrap().as_ptr()).elem }
    }

    /// Same as [Tree::peek], but returns a mutable reference instead
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(3);
    /// assert_eq!(tree.peek_mut(), &mut 3);
    /// ```
    pub fn peek_mut(&mut self) -> &mut T {
        if self.root.is_none() {
            panic!("Tried to peek mut on an empty tree");
        }
        unsafe { &mut (*self.current.unwrap().as_ptr()).elem }
    }

    /// Peek on 'current'.childs\[index\]
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// assert_eq!(tree.peek_child(2), &3);
    /// ```
    ///
    /// # Panics
    /// This method will panic if index >= tree.childs_len()
    pub fn peek_child(&self, index: usize) -> &T {
        if self.root.is_none() {
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
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(0);
    /// tree.push_iter(vec![1, 2, 3]);
    /// assert_eq!(tree.peek_child_mut(2), &mut 3);
    /// ```
    ///
    /// # Panics
    /// This method will panic if index >= tree.childs_len()
    pub fn peek_child_mut(&mut self, index: usize) -> &mut T {
        if self.root.is_none() {
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
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(10);
    /// tree.push_iter(vec![1, 2, 3]);
    /// assert_eq!(tree.childs_len(), 3);
    /// ```
    pub fn childs_len(&self) -> usize {
        if self.root.is_none() {
            panic!("Tried to call childs_len on an empty tree");
        }
        unsafe { (*self.current.unwrap().as_ptr()).childs.len() }
    }

    /// Return an iterator over the elements of current
    pub fn iter_childs(&self) -> ChildIterator<'_, T> {
        if self.root.is_none() {
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
        if self.root.is_none() {
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
    /// # use gtree::Tree;
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
        if self.root.is_none() || other.root.is_none() {
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
    /// # use gtree::Tree;
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
        if self.root.is_none() {
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
    /// # use gtree::Tree;
    /// let tree = Tree::from_element(5);
    /// let cursor = tree.cursor();
    /// assert_eq!(cursor.peek(), &5);
    /// ```
    pub fn cursor(&self) -> Cursor<'_, T> {
        if self.root.is_none() {
            panic!("Tried to call cursor on an empty tree");
        }

        Cursor {
            current: self.current.unwrap(),
            _boo: PhantomData,
        }
    }

    /// Return a [Cursor] pointing at 'root'
    ///
    /// # Examples
    /// ```
    /// # use gtree::Tree;
    /// let mut tree = Tree::from_element(3);
    /// tree.push(10);
    /// tree.navigate_to(0);
    /// let cursor = tree.cursor_root();
    /// assert_eq!(cursor.peek(), &3);
    /// ```
    pub fn cursor_root(&self) -> Cursor<'_, T> {
        if self.root.is_none() {
            panic!("Tried to call cursor_root on an empty tree");
        }

        Cursor {
            current: self.root.unwrap(),
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
    current: ChildLink<T>,
    i: usize,
    _boo: PhantomData<&'a T>,
    len: usize,
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

/// Recursive function to clone the tree under cursor
fn clone_rec<T>(
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
            let ret = clone_rec(cursor, new_tree, tree);
            if ret.is_some() {
                res = ret;
            }
            new_tree.ascend();
            cursor.ascend();
        }
    }

    res
}

impl<T> Clone for Tree<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        if self.root.is_none() {
            panic!("Tried to call clone on an empty tree");
        }

        let mut cursor = self.cursor_root();
        let mut new_tree = Self::from_element(cursor.peek().clone());
        let new_current = clone_rec(&mut cursor, &mut new_tree, self);
        new_tree.current = new_current;
        new_tree
    }
}

impl<T> Drop for Node<T> {
    fn drop(&mut self) {
        for child in self.childs.iter() {
            unsafe {
                let _ = Box::from_raw(child.as_ptr());
            }
        }
    }
}

impl<T> Drop for Tree<T> {
    fn drop(&mut self) {
        self.current = None;
        if let Some(root) = self.root {
            unsafe {
                let _ = Box::from_raw(root.as_ptr());
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

        // Dumby function to consume and drop tree
        fn foo<T>(_tree: Tree<T>) {}
        foo(tree);
        assert_eq!(split_tree.peek(), &2);
    }

    #[test]
    #[should_panic(expected = "Tried to call split with index 3 but current has only 0 childs")]
    fn split_panic() {
        let mut tree = Tree::from_element(0);
        tree.split(3);
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
        // foo function to consume tree to drop it
        fn foo(_t: Tree<i32>) {}
        foo(tree);

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
}
