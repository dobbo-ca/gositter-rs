/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

pub struct RingBuffer<T> {
	data: Vec<T>,
	i: usize,
}

impl<T> RingBuffer<T> {
	pub fn new(capacity: usize) -> Self {
		Self {
			data: 