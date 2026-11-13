/*
 * Copyright (c) 2024 Qualcomm Innovation Center, Inc. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
*/

#include "fastcv_hal_utils.hpp"

const char* getFastCVErrorString(int status)
{
    switch(status)
    {
        case FASTCV_SUCCESS: return "Successful";
        case FASTCV_EFAIL: return "General failure";
        case FASTCV_EUNALIGNPARAM: return "Unaligned pointer parameter";
        case FASTCV_EBADPARAM: return "Bad parameters";
        case FASTCV_EINVALSTATE: return "Called at invalid state";
        case FASTCV_ENORES: return "Insufficient resources, memory, thread, etc";
        case FASTCV_EUNSUPPORTED: return "Unsupported feature";
        case FASTCV_EHWQDSP: return "Hardware QDSP failed to respond";
        case FASTCV_EHWGPU: return "Hardware GPU failed to respond";
        default: return "Unknown FastCV Error";
    }
}

const char* borderToString(int border)
{
    switch (border)
    {
        case 0: return "BORDER_CONSTANT";
        case 1: return "BORDER_REPLICATE";
        case 2: return "BORDER_REFLECT";
        case 3: return "BORDER_WRAP";
        case 4: return "BORDER_REFLECT_101";
        case 5: return "BORDER_TRANSPARENT";
        default: return "Unknown border type";
    }
}

const char* interpolationToString(int interpolation)
{
    switch (in