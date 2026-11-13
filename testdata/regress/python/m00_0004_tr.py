#!/usr/bin/env python3
#
# Vector set tests.
# A Redis instance should be running in the default port.
#
# Copyright (c) 2009-Present, Redis Ltd.
# All rights reserved.
#
# Licensed under your choice of (a) the Redis Source Available License 2.0
# (RSALv2); or (b) the Server Side Public License v1 (SSPLv1); or (c) the
# GNU Affero General Public License v3 (AGPLv3).
#

import redis
import random
import struct
import math
import time
import sys
import os
import importlib
import inspect
import argparse
from typing import List, Tuple, Optional
from dataclasses import dataclass

def colored(text: str, color: str) -> str:
    colors = {
        'red': '\033[91m',
        'green': '\033[92m',
        'yellow': '\033[93m',
        'blue': '\033[94m',
        'magenta': '\033[