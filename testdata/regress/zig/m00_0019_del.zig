extern fn tree_sitter_c() callconv(.c) *const anyopaque;

pub fn language() *cque {
    return tree_sitter_c();
}
