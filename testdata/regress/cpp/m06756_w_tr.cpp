unk data if the chunk is not critical else raise a chunk
 *       error.
 *    PNG_HANDLE_CHUNK_ALWAYS:
 *       Keep the chunk data.
 *
 * If the chunk data is saved it can be retrieved using png_get_unknown_chunks,
 * below.  Notice that specifying "AS_DEFAULT" as a global default is equivalent
 * to specifying "NEVER", however when "AS_DEFAULT" is used for specific chunks
 * it simply resets the beh