ed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/
use crate::{
	constants::get_default_user_agent,
	log,
	util::errors::{self, WrappedError},
};
use bytes::Bytes;
use core::panic;
use futures::stream::TryStreamExt;
use http::{
	header::{HeaderName, HeaderValue, CONTENT_LENGTH},
