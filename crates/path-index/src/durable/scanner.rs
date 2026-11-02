/*
 * CDDL HEADER START
 *
 * The contents of this file are subject to the terms of the
 * Common Development and Distribution License (the "License").
 * You may not use this file except in compliance with the License.
 * See LICENSE.txt included in this distribution for the specific
 * language governing permissions and limitations under the License.
 * When distributing Covered Code, include this CDDL HEADER in each
 * file and include the License file at LICENSE.txt.
 * If applicable, add the following below this CDDL HEADER, with the
 * fields enclosed by brackets "[]" replaced with your own identifying
 * information: Portions Copyright [yyyy] [name of copyright owner]
 *
 * CDDL HEADER END
 */
/*
 * Copyright (c) 2008, 2025, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2017, 2020, Chris Fraire <cfraire@me.com>.
 * Portions Copyright (c) 2026, swchen44.
 * Derived behavior and source provenance: OpenGrok 1.14.20,
 * commit 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3.
 */
// SPDX-License-Identifier: CDDL-1.0
//! Restricted source path discovery; no source content or SCM ownership.
use super::*;
use std::{ffi::CStr, os::unix::fs::MetadataExt};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Stamp {
    path: String,
    dev: u64,
    ino: u64,
    mode: u32,
    size: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
fn stamp(path: String, m: &std::fs::Metadata) -> Stamp {
    Stamp {
        path,
        dev: m.dev(),
        ino: m.ino(),
        mode: m.mode(),
        size: m.size(),
        mtime: m.mtime(),
        mtime_ns: m.mtime_nsec(),
        ctime: m.ctime(),
        ctime_ns: m.ctime_nsec(),
    }
}
struct Directory(*mut libc::DIR);
impl Drop for Directory {
    fn drop(&mut self) {
        unsafe {
            libc::closedir(self.0);
        }
    }
}
fn names(dir: &File, budget: usize) -> Result<Vec<String>, DurableError> {
    let dot = CString::new(".").unwrap();
    let fresh = open_at(dir, &dot, FLAGS_DIR, 0)?;
    // fdopendir owns the descriptor only on success.
    use std::os::fd::IntoRawFd;
    let fd = fresh.into_raw_fd();
    let ptr = unsafe { libc::fdopendir(fd) };
    if ptr.is_null() {
        unsafe {
            libc::close(fd);
        }
        return Err(io::Error::last_os_error().into());
    }
    let stream = Directory(ptr);
    let mut out = Vec::new();
    loop {
        unsafe {
            *libc::__error() = 0;
        }
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(0) {
                return Err(error.into());
            }
            break;
        }
        let raw = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if raw == b"." || raw == b".." {
            continue;
        }
        if out.len() >= budget {
            return Err(DurableError::LimitExceeded);
        }
        out.push(
            std::str::from_utf8(raw)
                .map_err(|_| DurableError::UnsafePath)?
                .to_owned(),
        );
    }
    out.sort_unstable();
    Ok(out)
}
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_' || b == b'.'
        })
}
#[derive(PartialEq, Eq)]
struct Observed {
    stamps: Vec<Stamp>,
    paths: Vec<String>,
    path_bytes: usize,
}
fn visit(
    dir: &File,
    relative: &str,
    project: &str,
    limits: IndexLimits,
    remaining: &mut usize,
    out: &mut Observed,
    depth: usize,
) -> Result<(), DurableError> {
    if depth > 128 {
        return Err(DurableError::LimitExceeded);
    }
    let before = dir.metadata()?;
    if before.mode() & 0o444 == 0 || before.mode() & 0o111 == 0 {
        return Err(DurableError::UnsafePath);
    }
    out.stamps.push(stamp(relative.to_owned(), &before));
    for name in names(dir, *remaining)? {
        *remaining = remaining
            .checked_sub(1)
            .ok_or(DurableError::LimitExceeded)?;
        let c_name = CString::new(name.as_bytes()).map_err(|_| DurableError::UnsafePath)?;
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe {
            libc::fstatat(
                dir.as_raw_fd(),
                c_name.as_ptr(),
                &mut st,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return Err(io::Error::last_os_error().into());
        }
        let kind = st.st_mode & libc::S_IFMT;
        if kind != libc::S_IFREG && kind != libc::S_IFDIR {
            return Err(DurableError::UnsafePath);
        }
        if name == ".git" {
            continue;
        }
        if name.starts_with('.') || !safe_name(&name) {
            return Err(DurableError::UnsafePath);
        }
        let rel = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };
        let logical = format!("/{project}/{rel}");
        if logical.len() > limits.max_path_bytes {
            return Err(DurableError::LimitExceeded);
        }
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if kind == libc::S_IFDIR {
                libc::O_DIRECTORY
            } else {
                0
            };
        let file = open_at(dir, &c_name, flags, 0)?;
        let meta = file.metadata()?;
        if meta.dev() != st.st_dev as u64
            || meta.ino() != st.st_ino
            || meta.mode() & u32::from(libc::S_IFMT) != u32::from(kind)
        {
            return Err(DurableError::SourceChanged);
        }
        if kind == libc::S_IFDIR {
            visit(&file, &rel, project, limits, remaining, out, depth + 1)?;
        } else {
            if meta.mode() & 0o444 == 0
                || ![".c", ".py", ".js"].iter().any(|ext| name.ends_with(ext))
            {
                return Err(DurableError::UnsafePath);
            }
            if out.paths.len() >= limits.max_documents {
                return Err(DurableError::LimitExceeded);
            }
            out.path_bytes = out
                .path_bytes
                .checked_add(logical.len())
                .ok_or(DurableError::LimitExceeded)?;
            if out.path_bytes > limits.max_total_path_bytes {
                return Err(DurableError::LimitExceeded);
            }
            out.stamps.push(stamp(rel, &meta));
            out.paths.push(logical);
        }
    }
    if stamp(relative.to_owned(), &dir.metadata()?) != stamp(relative.to_owned(), &before) {
        return Err(DurableError::SourceChanged);
    }
    Ok(())
}
impl DurablePathIndex {
    fn observe_source(&self) -> Result<Observed, DurableError> {
        self.check_source()?;
        let mut out = Observed {
            stamps: Vec::new(),
            paths: Vec::new(),
            path_bytes: 0,
        };
        let mut remaining = self
            .inner
            .limits
            .max_documents
            .checked_mul(8)
            .and_then(|n| n.checked_add(128))
            .ok_or(DurableError::LimitExceeded)?;
        visit(
            &self.source,
            "",
            &self.inner.project,
            self.inner.limits,
            &mut remaining,
            &mut out,
            0,
        )?;
        self.check_source()?;
        Ok(out)
    }
    #[cfg(feature = "fault-injection")]
    pub fn replace_from_source_with_precommit_observer(
        &self,
        expected: Option<u64>,
        cancellation: &CancellationToken,
        _observer: &mut dyn FnMut(),
    ) -> Result<u64, DurableError> {
        self.scan_replace(
            expected,
            cancellation,
            FaultPoint::ObserveBeforeCommit,
            Some(_observer),
        )
    }
    /// Indexes the documented constrained source layout. Caller coordinates source mutations.
    pub fn replace_from_source(
        &self,
        expected: Option<u64>,
        cancellation: &CancellationToken,
    ) -> Result<u64, DurableError> {
        self.scan_replace(expected, cancellation, FaultPoint::None, None)
    }
    fn scan_replace(
        &self,
        expected: Option<u64>,
        cancellation: &CancellationToken,
        fault: FaultPoint,
        observer: Option<&mut dyn FnMut()>,
    ) -> Result<u64, DurableError> {
        if self.indeterminate.load(Ordering::Acquire) {
            return Err(DurableError::PublicationIndeterminate);
        }
        let _writer = self.inner.writer.try_lock().map_err(|e| match e {
            std::sync::TryLockError::WouldBlock => DurableError::Busy,
            std::sync::TryLockError::Poisoned(_) => DurableError::Index(IndexError::LockPoisoned),
        })?;
        if self.indeterminate.load(Ordering::Acquire) {
            return Err(DurableError::PublicationIndeterminate);
        }
        cancellation.check()?;
        let original = self.observe_source()?;
        let check = || {
            if self.observe_source()? != original {
                return Err(DurableError::SourceChanged);
            }
            Ok(())
        };
        let paths = original.paths.iter().cloned().map(Ok).collect::<Vec<_>>();
        self.replace_locked(
            expected,
            paths.len(),
            paths,
            cancellation,
            fault,
            observer,
            Some(&check),
        )
    }
}
