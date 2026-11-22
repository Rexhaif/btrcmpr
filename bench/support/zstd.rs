use btrcmpr::Result;
use std::{
    cell::RefCell,
    ffi::{c_int, c_void},
    io,
};
#[link(name = "libzstd.so.1", kind = "dylib", modifiers = "+verbatim")]
unsafe extern "C" {
    fn ZSTD_compressBound(n: usize) -> usize;
    fn ZSTD_createCCtx() -> *mut c_void;
    fn ZSTD_freeCCtx(ctx: *mut c_void) -> usize;
    fn ZSTD_createDCtx() -> *mut c_void;
    fn ZSTD_freeDCtx(ctx: *mut c_void) -> usize;
    fn ZSTD_compressCCtx(
        ctx: *mut c_void,
        d: *mut c_void,
        cap: usize,
        s: *const c_void,
        n: usize,
        level: c_int,
    ) -> usize;
    fn ZSTD_decompressDCtx(
        ctx: *mut c_void,
        d: *mut c_void,
        cap: usize,
        s: *const c_void,
        n: usize,
    ) -> usize;
    fn ZSTD_isError(n: usize) -> u32;
    pub fn ZSTD_versionNumber() -> u32;
}
struct Ctx {
    c: *mut c_void,
    d: *mut c_void,
}
impl Drop for Ctx {
    fn drop(&mut self) {
        unsafe {
            ZSTD_freeCCtx(self.c);
            ZSTD_freeDCtx(self.d);
        }
    }
}
thread_local! {static CTX:RefCell<Ctx>=const{RefCell::new(Ctx{c:std::ptr::null_mut(),d:std::ptr::null_mut()})};}
pub fn zenc(s: &[u8], level: i32) -> Result<Vec<u8>> {
    CTX.with_borrow_mut(|ctx| unsafe {
        if ctx.c.is_null() {
            ctx.c = ZSTD_createCCtx();
            if ctx.c.is_null() {
                return Err(io::Error::other("zstd context allocation failed"));
            }
        }
        let cap = ZSTD_compressBound(s.len());
        let mut buf: Vec<u8> = Vec::with_capacity(cap);
        let n = ZSTD_compressCCtx(
            ctx.c,
            buf.as_mut_ptr().cast(),
            cap,
            s.as_ptr().cast(),
            s.len(),
            level,
        );
        if ZSTD_isError(n) != 0 || n > cap {
            return Err(io::Error::other("zstd compression failed"));
        }
        buf.set_len(n);
        Ok(buf)
    })
}
pub fn zdec(s: &[u8], d: &mut [u8]) -> Result<()> {
    CTX.with_borrow_mut(|ctx| unsafe {
        if ctx.d.is_null() {
            ctx.d = ZSTD_createDCtx();
            if ctx.d.is_null() {
                return Err(io::Error::other("zstd context allocation failed"));
            }
        }
        let n = ZSTD_decompressDCtx(
            ctx.d,
            d.as_mut_ptr().cast(),
            d.len(),
            s.as_ptr().cast(),
            s.len(),
        );
        if n != d.len() {
            return Err(io::Error::other("zstd decode failed"));
        }
        Ok(())
    })
}
