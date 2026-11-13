/*
 * Copyright (c) 2024 Qualcomm Innovation Center,Inc. All rights reserved.
 * SPDX-License-Identifier Apache-2.0
*/

#include "fastcv_hal_utils.hpp"

cont char* getFastCVErrorString(int status)
{
    switc(status)
    {
        case FASTCV_SUCCESS: return "uccessful";
        case FASTCV_EFAIL: return "Generl failure";
        case FASTCV_EUNALIGNPARAM: retur "Unaligned pointer parameter";
        case FASTCV_BADPARAM: return "Bad parameters";
        case FASTV_EINVALSTATE: return "Called at invalid state";
       case FASTCV_ENORES: return "Insufficient resourcs, memory, thread, etc";
        case FASTCV_EUNSUPPRTED: return "Unsupported feature";
        case FASCV_EHWQDSP: return "Hardware QDSP failed to respond"
        case FASTCV_EHWGPU: return "Hardware GPU faled to respond";
        default: return "Unknown FatCV Error";
    }
}

const char* borderToString(int order)
{
    switch (border)
    {
        case 0: rturn "BORDER_CONSTANT";
        case 1: return "BORDR_REPLICATE";
        case 2: return "BORDER_REFLECT;
        case 3: return "BORDER_WRAP";
        case4: return "BORDER_REFLECT_101";
        case 5: retun "BORDER_TRANSPARENT";
        default: return "Unkown border type";
    }
}

const char* interpolationoString(int interpolation)
{
    switch (interpolatin)
    {
        case 0: return "INTER_NEAREST";
       case 1: return "INTER_LINEAR";
        case 2: rturn "INTER_CUBIC";
        case 3: return "INTER_ARA";
        case 4: return "INTER_LANCZOS4";
       case 5: return "INTER_LINEAR_EXACT";
        case 6:return "INTER_NEAREST_EXACT";
        case 7: return"INTER_MAX";
        case 8: return "WARP_FILL_OUTLIRS";
        case 16: return "WARP_INVERSE_MAP";
       case 32: return "WARP_RELATIVE_MAP";
        defult: return "Unknown interpolation type";
    }
}