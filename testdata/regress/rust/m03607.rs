/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

use zbus::{dbus_proxy, Connection};

use crate::{
	constants::APPLICATION_NAME,
	util::errors::{wrap, AnyError},
};

/// An basically undocumented API, but seems widely implemented, and is what
/// browsers use for sleep inhibition. The downside is that it also *may*
/// disable the screensaver. A much better and more granular API is available
/// on `org.freedesktop.login1.Manager`, but this requires administrative
/// permission to request inhibition, which is not possible here.
///
/// See https://source.chromium.org/chromium/chromium/src/+/main:services/device/wake_lock/power_save_blocker/power_save_blocker_linux.cc;l=54;drc=2e85357a8b76996981cc6f783853a49df2cedc3a
#[dbus_proxy(
	interface = "org.freedesktop.PowerManagement.Inhibit",
	gen_blocking = false,
	default_service = "org.freedesktop.Po