use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{ProcessControlBlockInner, block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    let detect = process_inner.deadlock_detect;
    if detect && mutex.is_locked() {
        // banker's algorithm: try to grant the request virtually
        let cur_tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        if !deadlock_check(&process_inner, cur_tid, ResKind::Mutex(mutex_id)) {
            trace!("kernel: sys_mutex_lock DEADLOCK DETECTED");
            return -(0xdead as isize);
        }
    }
    drop(process_inner);
    drop(process);
    mutex.lock();
    // record the resource allocation of the current task
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.ensure_mut(mutex_id);
    task_inner.mutex_alloc[mutex_id] = true;
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    // release the recorded resource allocation of the current task
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.ensure_mut(mutex_id);
    task_inner.mutex_alloc[mutex_id] = false;
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    // release the recorded resource allocation of the current task
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.ensure_sem(sem_id);
    if task_inner.sem_alloc[sem_id] > 0 {
        task_inner.sem_alloc[sem_id] -= 1;
    }
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    let detect = process_inner.deadlock_detect;
    drop(process_inner);
    if detect && sem.inner.exclusive_access().count <= 0 {
        // banker's algorithm: try to grant the request virtually
        let cur_tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        let process_inner = process.inner_exclusive_access();
        let safe = deadlock_check(&process_inner, cur_tid, ResKind::Sem(sem_id));
        drop(process_inner);
        if !safe {
            trace!("kernel: sys_semaphore_down DEADLOCK DETECTED");
            return -(0xdead as isize);
        }
    }
    sem.down();
    // record the resource allocation of the current task
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.ensure_sem(sem_id);
    task_inner.sem_alloc[sem_id] += 1;
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// Following the guide, deadlock detection only covers mutex/semaphore
/// syscalls of the calling process (condvar wait internally locks the
/// mutex directly, thus is neither recorded nor checked -- this mixed
/// usage is out of the scope of this chapter according to the guide).
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect {}", enabled);
    if enabled > 1 {
        return -1;
    }
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.deadlock_detect = enabled == 1;
    0
}

/// The kind of resource a thread is requesting, for deadlock detection
#[derive(Debug)]
enum ResKind {
    /// requesting mutex_list[id]
    Mutex(usize),
    /// requesting semaphore_list[id]
    Sem(usize),
}

/// Banker's-algorithm based deadlock detection.
///
/// Virtually grant the pending request of thread `cur_tid` for resource
/// `want`, then check whether the resulting global state is safe. Returns
/// `true` if safe (the request can be granted), `false` if unsafe
/// (granting it would deadlock; the syscall caller should return an error).
///
/// NOTE: this function reads (shared) `process_inner` as well as the inner
/// state of each task and each sync primitive. As long as no two mutable
/// references into the same `UPSafeCell` are held simultaneously this is
/// fine; all collected state is copied into local vectors first.
fn deadlock_check(process_inner: &ProcessControlBlockInner, cur_tid: usize, want: ResKind) -> bool {
    let m1 = process_inner.mutex_list.len();
    let m2 = process_inner.semaphore_list.len();
    let m = m1 + m2;
    // unify the resource numbering: mutex i -> i, sem j -> m1 + j
    // 1. enumerate threads and their current allocations
    let mut tids: Vec<usize> = Vec::new();
    let mut allocs: Vec<Vec<isize>> = Vec::new();
    for task in process_inner.tasks.iter().flatten() {
        let mut alloc = vec![0isize; m];
        // `res` is None for a thread that has exited but not been reaped
        // yet; it no longer holds resources, so skip it
        let tid = {
            let task_inner = task.inner_exclusive_access();
            if task_inner.res.is_none() {
                drop(task_inner);
                continue;
            }
            for (i, held) in task_inner.mutex_alloc.iter().enumerate() {
                if *held && i < m1 {
                    alloc[i] = 1;
                }
            }
            for (j, units) in task_inner.sem_alloc.iter().enumerate() {
                if j < m2 {
                    alloc[m1 + j] = (*units).max(0);
                }
            }
            task_inner.res.as_ref().unwrap().tid
        };
        tids.push(tid);
        allocs.push(alloc);
    }
    let n = tids.len();
    let pos_of = |tid: usize| tids.iter().position(|t| *t == tid);
    // 2. Available vector
    let mut available = vec![0isize; m];
    for (i, mu) in process_inner.mutex_list.iter().enumerate() {
        if let Some(mu) = mu {
            available[i] = if mu.is_locked() { 0 } else { 1 };
        }
    }
    for (j, sem) in process_inner.semaphore_list.iter().enumerate() {
        if let Some(sem) = sem {
            let count = sem.inner.exclusive_access().count;
            available[m1 + j] = count.max(0);
        }
    }
    // 3. Need matrix: from wait queues of blocked resources,
    //    plus the hypothetical request of the current thread
    let mut need: Vec<Vec<isize>> = (0..n).map(|_| vec![0isize; m]).collect();
    for (i, mu) in process_inner.mutex_list.iter().enumerate() {
        if let Some(mu) = mu {
            for tid in mu.wait_tids() {
                if let Some(p) = pos_of(tid) {
                    need[p][i] += 1;
                }
            }
        }
    }
    for (j, sem) in process_inner.semaphore_list.iter().enumerate() {
        if let Some(sem) = sem {
            // collect waiting tids first, then release the RefMut; a
            // waiting task with `res == None` (exited) is skipped
            let wait_tids: Vec<usize> = sem
                .inner
                .exclusive_access()
                .wait_queue
                .iter()
                .filter_map(|task| {
                    task.inner_exclusive_access()
                        .res
                        .as_ref()
                        .map(|res| res.tid)
                })
                .collect();
            for tid in wait_tids {
                if let Some(p) = pos_of(tid) {
                    need[p][m1 + j] += 1;
                }
            }
        }
    }
    // the requester's candidate allocation ("grant the request virtually")
    if let Some(p) = pos_of(cur_tid) {
        match want {
            ResKind::Mutex(i) => {
                if i < m1 {
                    need[p][i] += 1;
                }
            }
            ResKind::Sem(j) => {
                if j < m2 {
                    need[p][m1 + j] += 1;
                }
            }
        }
    }
    // 4. safety check (banker's algorithm)
    let mut work = available.clone();
    let mut finish = vec![false; n];
    loop {
        let mut progressed = false;
        for t in 0..n {
            if !finish[t] && (0..m).all(|r| need[t][r] <= work[r]) {
                for r in 0..m {
                    work[r] += allocs[t][r];
                }
                finish[t] = true;
                progressed = true;
            }
        }
        if !progressed {
            break;
        }
    }
    finish.iter().all(|f| *f)
}
