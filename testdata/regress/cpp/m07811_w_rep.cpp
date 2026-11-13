T_ATOP";
    case ALPHA_COMPOSITE_XOR:       return "XOR";
    case ALPHA_COMPOSITE_PLUS:      return "PLUS";
    default: CV_Error(Error::StsBadArg, "unknown operator");
    }
}

// Porter-Duff weights straight from the W3C table, independently of the fixed-point implementation.
static void referenceWeights(int op, double as, double ad, double& fa, double& fb)
{
    switch (op)
    {
    case ALPHA_COMPOSITE_CLEAR:     fa = 0;      fb = 0;      break;
    case ALPHA_COMPOSITE_SOURCE:    fa = 1;      fb = 0;      break;
    case ALPHA_COMPOSITE_DEST:      fa = 0;      fb = 1;      break;
    case ALPHA_COMPOSITE_OVER:      fa = 1;      fb = 1 - as; break;
    case ALPHA_COMPOSITE_DEST_OVER: fa = 1 - ad; fb = 1;      break;
    case ALPHA_COMPOSITE_IN:        fa = ad;     fb = 0;      break;
    case ALPHA_COMPOSITE_DEST_IN:   fa = 0;      fb = as;     break;
    case ALPHA_COMPOSITE_OUT:       fa = 1 - ad; fb = 0;      break;
    case ALPHA_COMPOSITE_DEST_OUT:  fa = 0;      fb = 1 - as; break;
    case ALPHA_COMPOSITE_ATOP:      fa = ad;     fb = 1 - as; break;
    case ALPHA_COMPOSITE_DEST_ATOP: fa = 1 - ad; fb = as;     break;
    case ALPHA_COMPOSITE_XOR:       fa = 1 - ad; fb = 1 - as; break;
    case ALPHA_COMPOSITE_PLUS:      fa = 1;      fb = 1;      break;
    default: CV_Error(Error::StBadArg, "unknown operator");
    }
}

// Takes straight-alpha input