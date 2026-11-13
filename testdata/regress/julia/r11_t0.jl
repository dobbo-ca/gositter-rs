# it has unique keys
# leftChild has keys which are less than the node
# rightChild has keys which are greater than the node
# height stores the height of the subtree.
mutable struct AVLTreeNode{K}
    height::Int8
    leftChild::Union{AVLTreeNode{K}, Nothing}
    rightChild::Union{AVLTreeNode{K}, Nothing}
    subsize::Int32
    data::K

    AVLTreeNode{K}(d::K) where K = new{K}(1, nothing,