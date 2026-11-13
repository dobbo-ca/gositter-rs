
pub enum Option<T> {
    None,
    Some(T),
}

pb enum Node<T: Item> {
    Internal {
        children: Vec<Tree<T>>,
        height: u16
    },
    #[attribute1]
    #[attribute2]
    Leaf {
        value: T
    }
}
