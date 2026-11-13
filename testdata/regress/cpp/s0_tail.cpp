led to respond";
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
    switch (interpolation)
    {
        case 0: return "INTER_NEAREST";
        case 1: return "INTER_LINEAR";
        case 2: return "INTER_CUBIC";
        case 3: return "INTER_AREA";
        case 4: return "INTER_LANCZOS4";
        case 5: return "INTER_LINEAR_EXACT";
        case 6: return "INTER_NEAREST_EXACT";
        case 7: return "INTER_MAX";
        case 8: return "WARP_FILL_OUTLIERS";
        case 16: return "WARP_INVERSE_MAP";
        case 32: return "WARP_RELATIVE_MAP";
        default: return "Unknown interpolation type";
    }
}