);
        __m128i xlane_permutes = _mm_cmpgt_epi8(_mm_set1_epi8(16), perm_vec1);
        __m128i xlane_res  = _mm_shuffle_epi8(ret_vec0, perm_vec1);
        /* Since we can't wrap twice, we can simply keep the later half exactly how it is instead of having to _also_
         * shuffle those values */
        __m128i latter_half = _mm_blendv_epi8(ret_vec1, xlane_res, xlane_permutes);
        ret_vec = _mm256_inserti128_si256(_mm256_castsi128_si256(ret_vec0), latter_half, 1);
    }

    return ret_vec;
}

#define CHUNKSIZE        chunksize_avx2
#define CHUNKCOPY        chunkcopy_avx2
#define CHUNKUNROLL      chunkunroll_avx2
#define CHUNKMEMSET      chunkmemset_avx2
#define CHUNKMEMSET_SAFE chunkmemset_safe_avx2

#include "chunkset_tpl.h"

#define INFLATE_FAST     inflate_fast_avx2

#include "inffast_tpl.h"

#endif
