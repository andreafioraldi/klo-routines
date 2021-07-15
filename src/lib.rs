use core::{ffi::c_void, mem::transmute, ops::FnMut, ptr::null_mut};
use std::cell::RefCell;
use libc;

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
    pub fn new() -> Self {
        unsafe {
            let mut instance = Self {
                running: core::mem::zeroed(),
                suspended: core::mem::zeroed(),
                yielded: None,
                finished: false,
            };
            instance.running.uc_stack.ss_sp = libc::malloc(libc::MINSIGSTKSZ);
            instance.running.uc_stack.ss_size = libc::MINSIGSTKSZ;
            instance.running.uc_link = null_mut();
            instance
        }
    }

    pub fn yield_(&mut self, value: T) {
        unsafe {
            self.yielded = Some(value);
            libc::swapcontext(&mut self.running, &mut self.suspended);
        }
    }

    pub fn finish(&mut self) {
        unsafe {
            self.yielded = None;
            self.finished = true;
            libc::swapcontext(&mut self.running, &mut self.suspended);
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
    pub fn new(func: &'a mut F) -> Self {
        unsafe {
            let mut instance = Self {
                ctx: KloContext::new(),
                func,
            };
            libc::getcontext(&mut instance.ctx.running);
            libc::makecontext(
                &mut instance.ctx.running,
                transmute(wrapper::<F, T> as extern "C" fn(_)),
                1,
                instance.func as *mut _ as *mut c_void,
                // &mut instance.ctx as *mut _ as *mut c_void,
            );
            instance
        }
    }

    pub fn resume(&mut self) -> Option<T> {
        if self.ctx.finished {
            return None;
        }
        CUR_KLO.with(|v| {
            *v.borrow_mut() = &mut self.ctx as *mut _ as *mut c_void;
        });
        unsafe {
            libc::swapcontext(&mut self.ctx.suspended, &mut self.ctx.running);
        }
        self.ctx.yielded.take()
    }
}

pub fn yield_<T>(value: T) {
    CUR_KLO.with(|v| {
        let ctx: &mut KloContext<T> = unsafe { transmute(*v.borrow()) };
        ctx.yield_(value)
    });
}

#[cfg(test)]
mod tests {
    use crate::{yield_, KloRoutine};

    #[test]
    fn it_works() {
        let mut cnt = 0;
        let mut func = || {
            for _ in 0..16 {
                yield_(cnt);
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
}
