// Copyright 2019  Sebastian Wiesner <sebastian@swsnr.de>

// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at

// 	http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations under
// the License.

//! [gethostname()][ghn] for all platforms.
//!
//! [ghn]: http://pubs.opengroup.org/onlinepubs/9699919799/functions/gethostname.html

#![deny(warnings, missing_docs, clippy::all)]

use std::ffi::OsString;
use std::io::Error;

/// Get the standard host name for the current machine.
///
/// # Platform-specific behavior
///
/// **posix:** Wraps the POSIX [`gethostname`]  function provided by `libc` in a
/// safe interface. Note that for `glibc` and `musl` this is implemented in terms
/// of [`uname`] and doesn't use the linux `gethostame` system call. Due to the way
/// we use it this can not fail in practice.
///
/// **windows:** Wraps windows [`GetComputerNameExW`] with `ComputerNamePhysicalDnsHostname`.
/// Due to the way we use it this can not fail in practice.
///
/// # Panics and why this function doesn't return a error.
///
/// _Theoretically_ this function can fail in a unexpected way and panic on both
/// Windows and POSIX compliant systems as neither of them defines that
/// there can not be other error reasons then the ones listed in the documentation
/// (e.g. POSIX list no error cases but all POSIX implementations have error cases
/// for invalid or to small input buffers).
///
/// In practice this can only fail with to small or invalid input buffers (on
/// both POSIX and Windows). But on both systems we make sure to have a buffer
/// large enough to fit the host name. On POSIX we use the systems `HOST_NAME_MAX`
/// setting to fit any possible hostname. On Windows we ask window for the size
/// of the buffer we need.
///
/// **Because of this it should be practically impossible to fail.** As such we
/// consider failure a bug and panic (please [open a issue] if you ever run
/// into this on any operating system).
///
/// [`uname`]: https://pubs.opengroup.org/onlinepubs/9699919799/functions/uname.html
/// [`gethostname`]: http://pubs.opengroup.org/onlinepubs/9699919799/functions/gethostname.html
/// [`GetComputerNameExW`]: https://docs.microsoft.com/en-us/windows/desktop/api/sysinfoapi/nf-sysinfoapi-getcomputernameexw
/// [open a issue]: https://github.com/lunaryorn/gethostname.rs/issues
pub fn gethostname() -> OsString {
    gethostname_impl()
}

#[cfg(not(windows))]
fn gethostname_impl() -> OsString {
    use libc::{c_char, sysconf, _SC_HOST_NAME_MAX};
    use std::os::unix::ffi::OsStringExt;
    // Get the maximum size of host names on this system, and account for the
    // trailing NUL byte.
    let hostname_max = unsafe { sysconf(_SC_HOST_NAME_MAX) };
    let mut buffer = vec![0 as u8; (hostname_max as usize) + 1];
    let returncode = unsafe { libc::gethostname(buffer.as_mut_ptr() as *mut c_char, buffer.len()) };
    if returncode != 0 {
        // There are no reasonable failures, so lets panic
        panic!(
            "gethostname failed: {}
    Please report an issue to <https://github.com/lunaryorn/gethostname.rs/issues>!",
            Error::last_os_error()
        );
    }
    // We explicitly search for the trailing NUL byte and cap at the buffer
    // length: If the buffer's too small (which shouldn't happen since we
    // explicitly use the max hostname size above but just in case) POSIX
    // doesn't specify whether there's a NUL byte at the end, so if we didn't
    // check we might read from memory that's not ours.
    let end = buffer
        .iter()
        .position(|&b| b == 0)
        .unwrap_or_else(|| buffer.len());
    buffer.resize(end, 0);
    OsString::from_vec(buffer)
}

#[cfg(windows)]
fn gethostname_impl() -> OsString {
    use std::os::windows::ffi::OsStringExt;
    use winapi::ctypes::{c_ulong, wchar_t};
    use winapi::um::sysinfoapi::{ComputerNamePhysicalDnsHostname, GetComputerNameExW};

    let mut buffer_size: c_ulong = 0;

    unsafe {
        // This call always fails with ERROR_MORE_DATA, because we pass NULL to
        // get the required buffer size.
        GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            std::ptr::null_mut(),
            &mut buffer_size,
        )
    };

    let mut buffer = vec![0 as wchar_t; buffer_size as usize];
    let returncode = unsafe {
        GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            buffer.as_mut_ptr() as *mut wchar_t,
            &mut buffer_size,
        )
    };
    // GetComputerNameExW returns a non-zero value on success!
    if returncode == 0 {
        panic!(
            "GetComputerNameExW failed to read hostname: {}
Please report this issue to <https://github.com/lunaryorn/gethostname.rs/issues>!",
            Error::last_os_error()
        );
    }

    let end = buffer
        .iter()
        .position(|&b| b == 0)
        .unwrap_or_else(|| buffer.len());
    OsString::from_wide(&buffer[0..end])
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use std::process::Command;

    #[test]
    fn gethostname_matches_system_hostname() {
        let output = Command::new("hostname")
            .output()
            .expect("failed to get hostname");
        let hostname = String::from_utf8_lossy(&output.stdout);
        // Convert both sides to lowercase; hostnames are case-insensitive
        // anyway.
        assert_eq!(
            super::gethostname().into_string().unwrap().to_lowercase(),
            hostname.trim_end().to_lowercase()
        );
    }

    #[test]
    #[ignore]
    fn gethostname_matches_fixed_hostname() {
        assert_eq!(
            super::gethostname().into_string().unwrap().to_lowercase(),
            "hostname-for-testing"
        );
    }
}
