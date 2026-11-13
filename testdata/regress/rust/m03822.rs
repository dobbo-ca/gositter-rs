/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

use std::io;

use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use libc::c_int;

use crate::constants::TUNNEL_ACTIVITY_NAME;

extern "C" {
	pub fn IOPMAssertionCreateWithName(
		assertion_type: CFStringRef,
		assertion_level: u32,
		assertion_name: CFStringRef,
		assertion_id: &mut u32,
	) -> c_int;

	pub fn IOPMAssertionRelease(assertion_id: u32) -> c_int;
}

const NUM_ASSERTIONS: us