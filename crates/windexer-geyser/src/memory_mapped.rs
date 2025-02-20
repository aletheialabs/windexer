// crates/windexer-geyser/src/memory_mapped.rs
use nix::{
    libc,
    sys::{
        mman::{mmap, munmap, mlock, mlockall, MsyncFlags, ProtFlags, MapFlags, MlockAllFlags},
        stat::fstat,
    },
    unistd::Pid,
};
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::{
    fs::File,
    io::{Error, ErrorKind, Result},
    os::unix::prelude::{AsRawFd, FromRawFd, RawFd},
    ptr::{self, NonNull},
    sync::atomic::{AtomicBool, Ordering},
};
use thiserror::Error;

const ACCOUNT_SIZE: usize = 136;
const MMAP_ALIGN: usize = 4096;

#[derive(Error, Debug)]
pub enum MemoryMapError {
    #[error("Memory mapping failed: {0}")]
    MapFailed(#[from] nix::Error),
    #[error("Invalid alignment for address 0x{0:x}")]
    AlignmentError(u64),
    #[error("Validator process not found")]
    ValidatorNotFound,
    #[error("Memory synchronization failed")]
    SyncError,
}

#[repr(C)]
pub struct ValidatorMemoryMap {
    addr: NonNull<libc::c_void>,
    size: usize,
    fd: RawFd,
    pid: u32,
    locked: AtomicBool,
}

impl ValidatorMemoryMap {
    /// Maps the validator's memory region containing accounts
    /// # Safety
    /// Requires CAP_SYS_PTRACE and CAP_SYS_ADMIN capabilities
    pub unsafe fn new(pid: u32, start_addr: u64, size: usize) -> Result<Self, MemoryMapError> {
        // Validate alignment
        if start_addr % MMAP_ALIGN as u64 != 0 || size % MMAP_ALIGN != 0 {
            return Err(MemoryMapError::AlignmentError(start_addr));
        }

        let proc_mem = format!("/proc/{}/mem", pid);
        let file = File::open(proc_mem).map_err(|e| {
            Error::new(
                ErrorKind::PermissionDenied,
                format!("Failed to open /proc/{}/mem: {}", pid, e),
            )
        })?;

        let fd = file.as_raw_fd();
        let stat = fstat(fd).map_err(|e| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Failed to stat memory file: {}", e),
            )
        })?;

        let addr = mmap(
            ptr::null_mut(),
            size,
            ProtFlags::PROT_READ,
            MapFlags::MAP_SHARED,
            fd,
            start_addr as i64,
        )?;

        mlockall(MlockAllFlags::MLOCK_FUTURE).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Failed to lock memory: {}", e),
            )
        })?;

        Ok(Self {
            addr: NonNull::new(addr).ok_or(MemoryMapError::MapFailed(nix::Error::EINVAL))?,
            size,
            fd,
            pid,
            locked: AtomicBool::new(true),
        })
    }

    #[inline]
    pub unsafe fn get_account(&self, offset: usize) -> Result<&Account> {
        if offset + ACCOUNT_SIZE > self.size {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Offset exceeds mapped region",
            ));
        }
        

        let ptr = self.addr.as_ptr().add(offset) as *const Account;
        Ok(&*ptr)
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    pub unsafe fn find_account_avx(
        &self,
        pubkey: &Pubkey,
    ) -> Result<Option<(&Account, usize)>> {
        use std::arch::x86_64::{
            __m256i, _mm256_cmpeq_epi8, _mm256_load_si256, _mm256_movemask_epi8,
        };

        let needle: __m256i = std::mem::transmute(*pubkey.as_array());
        let mut offset = 0;

        while offset + 32 <= self.size {
            let haystack = self.addr.as_ptr().add(offset) as *const __m256i;
            let cmp = _mm256_cmpeq_epi8(needle, _mm256_load_si256(haystack));
            let mask = _mm256_movemask_epi8(cmp) as u32;

            if mask == 0xffffffff {
                return Ok(Some((
                    self.get_account(offset - 32 + std::mem::size_of::<Pubkey>())?,
                    offset,
                )));
            }

            offset += 32;
        }

        Ok(None)
    }

    pub fn sync(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            nix::sys::mman::msync(
                self.addr.as_ptr() as *mut libc::c_void,
                self.size,
                MsyncFlags::MS_SYNC,
            )
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        }
        Ok(())
    }

    pub fn lock(&self) -> Result<()> {
        if self.locked.load(Ordering::Relaxed) {
            return Ok(());
        }

        mlock(self.addr.as_ptr(), self.size).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Failed to lock memory: {}", e),
            )
        })?;
        self.locked.store(true, Ordering::Release);
        Ok(())
    }

    pub fn unlock(&self) -> Result<()> {
        if !self.locked.load(Ordering::Relaxed) {
            return Ok(());
        }

        nix::sys::mman::munlock(self.addr.as_ptr(), self.size).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Failed to unlock memory: {}", e),
            )
        })?;
        self.locked.store(false, Ordering::Release);
        Ok(())
    }
}

impl Drop for ValidatorMemoryMap {
    fn drop(&mut self) {
        if self.locked.load(Ordering::Relaxed) {
            let _ = self.unlock();
        }

        unsafe {
            munmap(self.addr.as_ptr(), self.size).ok();
        }
    }
}

unsafe impl Send for ValidatorMemoryMap {}
unsafe impl Sync for ValidatorMemoryMap {}
