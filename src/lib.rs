use core::{ffi::c_void, mem::transmute, ops::FnMut, ptr::null_mut};
use libc;
#[cfg(all(unix, not(any(target_os = "macos", target_is = "ios"))))]
use libc::{getcontext, makecontext, swapcontext};
use std::cell::RefCell;

// swapcontext and getcontext are not exported through rust's libc crate on macos.
#[cfg(any(target_os = "macos", target_is = "ios"))]
extern "C" {
    fn makecontext(
        ucp: *mut libc::ucontext_t,
        func: extern "C" fn(),
        argc: libc::c_int,
        data: *mut libc::c_void,
    );
    fn getcontext(ucp: *mut libc::ucontext_t) -> libc::c_int;

    fn swapcontext(oucp: *mut libc::ucontext_t, ucp: *mut libc::ucontext_t) -> libc::c_int;
}

thread_local! {
    static CUR_KLO: RefCell<*mut c_void> = RefCell::new(null_mut());
}

extern "C" fn wrapper<F, T>(func_ptr: *mut c_void)
where
    F: FnMut(),
{
    let func: &mut F = unsafe { transmute(func_ptr) };
    func();
    CUR_KLO.with(|v| {
        let ctx: &mut KloContext<T> = unsafe { transmute(*v.borrow()) };
        if !ctx.finished {
            ctx.finish();
        }
    });
}

pub struct KloContext<T> {
    running: libc::ucontext_t,
    suspended: libc::ucontext_t,
    yielded: Option<T>,
    finished: bool,
}

impl<T> KloContext<T> {
    pub fn new(size: usize) -> Self {
        assert!(size >= libc::MINSIGSTKSZ);
        unsafe {
            let mut instance = Self {
                running: core::mem::zeroed(),
                suspended: core::mem::zeroed(),
                yielded: None,
                finished: false,
            };
            instance.running.uc_stack.ss_sp = libc::malloc(size);
            instance.running.uc_stack.ss_size = size;
            instance.running.uc_link = null_mut();
            instance
        }
    }

    pub fn yield_(&mut self, value: T) {
        unsafe {
            self.yielded = Some(value);
            if swapcontext(&mut self.running, &mut self.suspended) == -1 {
                libc::perror(b"swapcontext\0" as *const _ as *const libc::c_char);
                panic!("swapcontext failed");
            }
        }
    }

    pub fn finish(&mut self) {
        unsafe {
            self.yielded = None;
            self.finished = true;
            if swapcontext(&mut self.running, &mut self.suspended) == -1 {
                libc::perror(b"swapcontext\0" as *const _ as *const libc::c_char);
                panic!("swapcontext failed");
            }
        }
    }
}

impl<T> Drop for KloContext<T> {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.running.uc_stack.ss_sp);
            libc::free(self.suspended.uc_stack.ss_sp);
        }
    }
}

pub struct KloRoutine<'a, F, T> {
    ctx: KloContext<T>,
    func: &'a mut F,
}

impl<'a, F, T> KloRoutine<'a, F, T>
where
    F: FnMut(),
{
    pub fn with_stack_size(func: &'a mut F, size: usize) -> Self {
        unsafe {
            let mut instance = Self {
                ctx: KloContext::new(size),
                func,
            };

            let ss_sp = instance.ctx.running.uc_stack.ss_sp;
            let ss_size = instance.ctx.running.uc_stack.ss_size;
            let uc_link = instance.ctx.running.uc_link;

            if getcontext(&mut instance.ctx.running) != 0 {
                libc::perror(b"getcontext\0" as *const _ as *const libc::c_char);
                panic!("getcontext failed");
            }

            instance.ctx.running.uc_stack.ss_sp = ss_sp;
            instance.ctx.running.uc_stack.ss_size = ss_size;
            instance.ctx.running.uc_link = uc_link;

            makecontext(
                &mut instance.ctx.running,
                transmute(wrapper::<F, T> as extern "C" fn(_)),
                1,
                instance.func as *mut _ as *mut c_void,
                // &mut instance.ctx as *mut _ as *mut c_void,
            );
            instance
        }
    }

    pub fn new(func: &'a mut F) -> Self {
        Self::with_stack_size(func, 16 * 1024 * 1024)
    }

    pub fn resume(&mut self) -> Option<T> {
        if self.ctx.finished {
            return None;
        }
        CUR_KLO.with(|v| {
            *v.borrow_mut() = &mut self.ctx as *mut _ as *mut c_void;
        });
        unsafe {
            if swapcontext(&mut self.ctx.suspended, &mut self.ctx.running) == -1 {
                libc::perror(b"swapcontext\0" as *const _ as *const libc::c_char);
                panic!("swapcontext failed");
            }
        }
        self.ctx.yielded.take()
    }
}

impl<'a, F, T> Iterator for KloRoutine<'a, F, T>
where
    F: FnMut(),
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.resume()
    }
}

pub fn yield_<T>(value: T) {
    CUR_KLO.with(|v| {
        let ctx: &mut KloContext<T> = unsafe { transmute(*v.borrow()) };
        ctx.yield_(value)
    });
}

pub fn flush<T>(value: T) {
    yield_(value)
}

#[cfg(test)]
mod tests {
    use crate::{flush, KloRoutine};

    #[test]
    fn it_works() {
        let mut cnt = 0;
        let mut func = || {
            for _ in 0..16 {
                flush(cnt);
                cnt += 1;
            }
        };
        let mut klo = KloRoutine::new(&mut func);

        // Is safe to move klo
        let mut move_fn = move || {
            for i in 0..16 {
                assert_eq!(Some(i), klo.resume());
            }
            assert_eq!(None, klo.resume());
        };

        move_fn();
    }

    #[test]
    fn iterator() {
        let mut cnt = 0;
        let mut func = || {
            for _ in 0..16 {
                flush(cnt);
                cnt += 1;
            }
        };
        let mut klo = KloRoutine::new(&mut func);

        let mut i = 0;
        for n in &mut klo {
            assert_eq!(i, n);
            i += 1;
        }
    }
}
