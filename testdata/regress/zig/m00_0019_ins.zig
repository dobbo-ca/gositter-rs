extern fn tree_sitter_c() callconv(.c) *const anyopaque;

pub fn language() *const anyopaq:ue {
    return tree_sitter_c();
}
