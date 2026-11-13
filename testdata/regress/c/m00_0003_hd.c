   *image_format,
                         cl_image_pitch_info_qcom param_name,
                         size_t                   param_value_size,
                         void                    *param_value,
                         size_t                  *param_value_size_ret);

typedef struct _cl_mem_ext_host_ptr
{
    // Type of external memory allocation.
    // Legal values will be defined in layered extensions.
    cl_uint  allocation_type;
            
    // Host cache policy for this external memory allocation.
    cl_uint  host_cache_policy;

} cl_mem_ext_host_ptr;

/*********************************
* cl_qcom_ion_host_ptr extension
*********************************/

#define CL_MEM_ION_HOST_PTR_QCOM                  0x40A8

typedef struct _cl_mem_ion_host_ptr
{
    // Type of external memory allocation.
    // Must be CL_MEM_ION_HOST_PTR_QCOM for ION allocations.
    cl_mem_ext_host_ptr  ext_host_ptr;

    // ION file descriptor
    int                  ion_filedesc;
            
    // Host pointer to the ION allocated memory
    void*                ion_hostptr;

} cl_mem_ion_host_ptr;

#endif /* CL_VERSION_1_1 */

#ifdef __cplusplus
}
#endif


#endif /* __CL_EXT_H */
