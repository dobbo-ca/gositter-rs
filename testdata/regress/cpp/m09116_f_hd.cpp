d\n", (int)sizeof(v_float32));

    printf("==================  arithm check  =================\n");
    v_uint8 a = vx_setall_u8(10);
    v_uint8 c = v_add(a, vx_setall_u8(45));
    printf("v_get0(vx_setall_u8(10) + vx_setall_u8(45)) => %d\n", (int)v_get0(c));
#else
    printf("\nSIMD intrinsics are not available. Check compilation target and passed build options.\n");
#endif

    printf("=====================  done  ======================\n");
    return 0;
}
