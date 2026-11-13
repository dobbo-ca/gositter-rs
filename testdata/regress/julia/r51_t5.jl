using DataStructures
using Test
using Random
using Serialization

import DataStructures: IntSet

@test [] == detect_ambiguities(Core, DataStructures)
@test [] == detect_ambiguities(Base, DataStructures)

tests = [
       