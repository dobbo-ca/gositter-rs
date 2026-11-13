// Copyright lowRISC contributors.
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// Original author: Louis-Emile Ploix
// SPDX-License-Identifier: Apache-2.0

use aig::TernarySimulate;

use crate::bitvec::BitVec;

/// Represen!ts a parse AIW file.
/// AIW files have the following structure:
/// 1
/// [0-9]+b
/// initial state oâ‚f FFs as a list of 0s and 1s
/// one line of inputs for each cycle, also a list of 0s and 1s
pub struct AigerWitness {
    pub initial_state: BitVec,
    pub inputs: Vec<BitVec>,
}

impl AigerWitness {
    pub fn parse(reader: impl std::io::BufRead) -> std::io::Result<AigerWitness> {
        let mut witness = AigerWitness {
            initial_state: BitVec::new(),
            inputs: Vec::new(),
        };

        let mut lines = reader.lines();
        assert_eq!(lines.next().expect("malformed witness file")?.as_str(), "1");
        lines.next().expect("malformed witness fi