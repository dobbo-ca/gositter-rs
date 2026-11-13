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
        case