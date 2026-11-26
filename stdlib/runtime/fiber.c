// Fibers: lightweight threads on the program's one OS thread, for std/async.
//
// Each fiber has a stack of its own (mmap'd, with a guard page below it), and
// runs until it waits: on a task, a channel, or a timer (sleep). Then the
// scheduler switches to the next ready fiber. Only one runs at a time, so qiq
// code needs no locks, and reference counts stay plain.
//
// The program's own stack is the main fiber. The few pieces of global state
// that belong to the code running (the pending exception, and the innermost
// guarded runtime call) are saved and restored on each switch.
//
// std/async builds Task, Job, and Channel on these primitives:
//   __fiber_spawn(body)   starts a fiber running a qiq `function(): void`
//   __fiber_current()     the running fiber, as a handle for __fiber_wake
//   __fiber_park()        waits until another fiber wakes this one
//   __fiber_wake(fiber)   makes a parked fiber ready to run again
//   __fiber_yield()       lets the other ready fibers run first
//   qiq_poll(fds, n, ms)  poll() for the runtime's I/O (sockets, pipes):
//                         other fibers run until a descriptor is ready
//   __fiber_failed(e)     notes a task's exception, reported at the end
//   __fiber_observed(e)   unless an await sees it first
// The compiled program calls qiq_fibers_finish at the end of its main code,
// and std/time's sleep and usleep park the fiber on a timer when other
// fibers exist (see qiq_fiber_sleep).

#include "qiq_runtime.h"

#include <errno.h>
#include <setjmp.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <poll.h>
#include <sys/epoll.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>

// From core.c: the innermost guarded runtime call (see QIQ_GUARDED).
jmp_buf *qiq_guard_get(void);
void qiq_guard_set(jmp_buf *guard);
// Defined in every compiled program (see qiq_raise in core.c).
extern void qiq_raise(int32_t kind, const char *message);

// Each fiber's stack: 1 MiB of address space, committed only as it's used.
#define QIQ_FIBER_STACK (1024 * 1024)
#define QIQ_FIBER_GUARD 4096
// Finished fibers' stacks kept for reuse.
#define QIQ_FIBER_POOL 64

typedef enum { QIQ_FIBER_READY, QIQ_FIBER_RUNNING, QIQ_FIBER_PARKED, QIQ_FIBER_DONE } QiqFiberState;

typedef struct QiqFiber QiqFiber;
struct QiqFiber {
    void *sp;            // the stack pointer saved when it's switched out
    char *stack;         // the mapping, guard page first; NULL for the main fiber
    void *body;          // the qiq function value it runs
    void *pending;       // qiq_pending_exception while it's switched out
    jmp_buf *guard;      // the innermost guarded call while it's switched out
    QiqFiberState state;
    // Counts its waits: a timer or I/O registration wakes it only if it's
    // still in the wait that made it (not a later one).
    uint64_t wait;
    struct QiqFiber *next;  // in the ready queue, or the free list
};

static QiqFiber qiq_main_fiber = {.state = QIQ_FIBER_RUNNING};
static QiqFiber *qiq_current = &qiq_main_fiber;
// Fibers ready to run, first in first out.
static QiqFiber *qiq_ready_head = NULL;
static QiqFiber *qiq_ready_tail = NULL;
// Fibers that have started and not ended, not counting the main fiber.
static int64_t qiq_alive = 0;
// Whether the main fiber is waiting for every other fiber to end.
static int qiq_main_waits_for_all = 0;
// A fiber that has ended, whose stack is freed by the next fiber to run (it
// can't free the stack it's running on).
static QiqFiber *qiq_finished = NULL;
// Ended fibers kept to be used again, with their stacks.
static QiqFiber *qiq_pool = NULL;
static int qiq_pool_size = 0;

// ---------------------------------------------------------------------------
// Switching stacks (x86-64, System V)
//
// qiq_fiber_switch(&from->sp, to->sp) saves the callee-saved registers and
// the SSE and x87 control words on the current stack, stores the stack
// pointer, then loads `to`'s and restores what it saved. A new fiber's stack
// is laid out so that this "returns" into qiq_fiber_start with the fiber in
// r12 and qiq_fiber_main in r13.

void qiq_fiber_switch(void **save, void *load);
void qiq_fiber_start(void);
void qiq_fiber_main(QiqFiber *fiber);

__asm__(
    ".text\n"
    ".globl qiq_fiber_switch\n"
    ".type qiq_fiber_switch,@function\n"
    "qiq_fiber_switch:\n"
    "    pushq %rbp\n"
    "    pushq %rbx\n"
    "    pushq %r12\n"
    "    pushq %r13\n"
    "    pushq %r14\n"
    "    pushq %r15\n"
    "    subq $8, %rsp\n"
    "    stmxcsr (%rsp)\n"
    "    fnstcw 4(%rsp)\n"
    "    movq %rsp, (%rdi)\n"
    "    movq %rsi, %rsp\n"
    "    ldmxcsr (%rsp)\n"
    "    fldcw 4(%rsp)\n"
    "    addq $8, %rsp\n"
    "    popq %r15\n"
    "    popq %r14\n"
    "    popq %r13\n"
    "    popq %r12\n"
    "    popq %rbx\n"
    "    popq %rbp\n"
    "    ret\n"
    ".size qiq_fiber_switch, .-qiq_fiber_switch\n"
    ".globl qiq_fiber_start\n"
    ".type qiq_fiber_start,@function\n"
    "qiq_fiber_start:\n"
    "    movq %r12, %rdi\n"
    "    andq $-16, %rsp\n"
    "    call *%r13\n"
    "    ud2\n"
    ".size qiq_fiber_start, .-qiq_fiber_start\n");

// ---------------------------------------------------------------------------
// Timers: fibers asleep until a time, in a binary heap by deadline.

typedef struct {
    uint64_t deadline;  // CLOCK_MONOTONIC nanoseconds
    uint64_t order;     // ties wake in the order they slept
    QiqFiber *fiber;
    uint64_t wait;      // the fiber's wait it ends
} QiqTimer;

static QiqTimer *qiq_timers = NULL;
static size_t qiq_timer_count = 0;
static size_t qiq_timer_capacity = 0;
static uint64_t qiq_timer_order = 0;

static uint64_t qiq_now(void) {
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return (uint64_t)now.tv_sec * 1000000000u + (uint64_t)now.tv_nsec;
}

static int qiq_timer_before(const QiqTimer *a, const QiqTimer *b) {
    return a->deadline < b->deadline || (a->deadline == b->deadline && a->order < b->order);
}

static void qiq_timer_push(uint64_t deadline, QiqFiber *fiber, uint64_t wait) {
    if (qiq_timer_count == qiq_timer_capacity) {
        qiq_timer_capacity = qiq_timer_capacity ? qiq_timer_capacity * 2 : 16;
        QiqTimer *grown = realloc(qiq_timers, qiq_timer_capacity * sizeof *qiq_timers);
        if (grown == NULL) qiq_fatal("out of memory");
        qiq_timers = grown;
    }
    size_t at = qiq_timer_count++;
    qiq_timers[at] = (QiqTimer){deadline, qiq_timer_order++, fiber, wait};
    while (at > 0) {
        size_t above = (at - 1) / 2;
        if (!qiq_timer_before(&qiq_timers[at], &qiq_timers[above])) break;
        QiqTimer held = qiq_timers[at];
        qiq_timers[at] = qiq_timers[above];
        qiq_timers[above] = held;
        at = above;
    }
}

static QiqTimer qiq_timer_pop(void) {
    QiqTimer popped = qiq_timers[0];
    qiq_timers[0] = qiq_timers[--qiq_timer_count];
    size_t at = 0;
    for (;;) {
        size_t first = at, left = 2 * at + 1, right = left + 1;
        if (left < qiq_timer_count && qiq_timer_before(&qiq_timers[left], &qiq_timers[first])) first = left;
        if (right < qiq_timer_count && qiq_timer_before(&qiq_timers[right], &qiq_timers[first])) first = right;
        if (first == at) break;
        QiqTimer held = qiq_timers[at];
        qiq_timers[at] = qiq_timers[first];
        qiq_timers[first] = held;
        at = first;
    }
    return popped;
}

// ---------------------------------------------------------------------------
// The scheduler

static void qiq_make_ready(QiqFiber *fiber);

// Wakes `fiber` if it's still parked in wait `wait`.
static void qiq_end_wait(QiqFiber *fiber, uint64_t wait) {
    if (fiber->state == QIQ_FIBER_PARKED && fiber->wait == wait) qiq_make_ready(fiber);
}

// ---------------------------------------------------------------------------
// Waiting for I/O: an epoll instance, made when first needed, with each
// descriptor a parked fiber waits on registered to wake it.

typedef struct {
    QiqFiber *fiber;
    uint64_t wait;
} QiqIoWaiter;

static int qiq_epoll = -1;
// Registrations: descriptors fibers are waiting on.
static int64_t qiq_io_waiting = 0;

// Waits up to `timeout_ms` (-1: no limit) for registered descriptors, and
// wakes the fibers waiting on those that are ready.
static void qiq_poll_io(int timeout_ms) {
    struct epoll_event events[64];
    int count;
    do {
        count = epoll_wait(qiq_epoll, events, 64, timeout_ms);
    } while (count < 0 && errno == EINTR);
    for (int index = 0; index < count; index++) {
        QiqIoWaiter *waiter = events[index].data.ptr;
        qiq_end_wait(waiter->fiber, waiter->wait);
    }
}

static void qiq_make_ready(QiqFiber *fiber) {
    fiber->state = QIQ_FIBER_READY;
    fiber->next = NULL;
    if (qiq_ready_tail != NULL) qiq_ready_tail->next = fiber;
    else qiq_ready_head = fiber;
    qiq_ready_tail = fiber;
}

// Frees (or keeps for reuse) the stack of a fiber that has ended.
static void qiq_reclaim_finished(void) {
    QiqFiber *fiber = qiq_finished;
    if (fiber == NULL) return;
    qiq_finished = NULL;
    if (qiq_pool_size < QIQ_FIBER_POOL) {
        fiber->next = qiq_pool;
        qiq_pool = fiber;
        qiq_pool_size++;
        return;
    }
    munmap(fiber->stack, QIQ_FIBER_STACK + QIQ_FIBER_GUARD);
    free(fiber);
}

// Runs `to` in place of the current fiber, carrying each fiber's own
// pending exception and guarded call with it.
static void qiq_switch_to(QiqFiber *to) {
    QiqFiber *from = qiq_current;
    if (to == from) {
        to->state = QIQ_FIBER_RUNNING;
        return;
    }
    from->pending = qiq_pending_exception;
    from->guard = qiq_guard_get();
    qiq_current = to;
    to->state = QIQ_FIBER_RUNNING;
    qiq_pending_exception = to->pending;
    to->pending = NULL;
    qiq_guard_set(to->guard);
    qiq_fiber_switch(&from->sp, to->sp);
    // Back in `from`: whichever fiber ran before may have ended.
    qiq_reclaim_finished();
}

// The fiber to run next: the first ready one, or, when none is, the first
// woken by I/O or a timer (waiting until one is). NULL when every fiber is
// waiting on something only another fiber could do: a deadlock.
static QiqFiber *qiq_next_fiber(void) {
    for (;;) {
        if (qiq_ready_head != NULL) {
            QiqFiber *fiber = qiq_ready_head;
            qiq_ready_head = fiber->next;
            if (qiq_ready_head == NULL) qiq_ready_tail = NULL;
            return fiber;
        }
        if (qiq_timer_count == 0 && qiq_io_waiting == 0) return NULL;
        uint64_t now = qiq_now();
        uint64_t until = qiq_timer_count > 0 && qiq_timers[0].deadline > now ? qiq_timers[0].deadline - now : 0;
        if (qiq_io_waiting > 0) {
            // Milliseconds, rounded up so a timer isn't woken early.
            int timeout_ms = qiq_timer_count == 0 ? -1 : (int)((until + 999999u) / 1000000u);
            qiq_poll_io(timeout_ms);
        } else if (until > 0) {
            struct timespec duration = {.tv_sec = (time_t)(until / 1000000000u), .tv_nsec = (long)(until % 1000000000u)};
            while (nanosleep(&duration, &duration) != 0 && errno == EINTR) {
            }
        }
        now = qiq_now();
        while (qiq_timer_count > 0 && qiq_timers[0].deadline <= now) {
            QiqTimer timer = qiq_timer_pop();
            qiq_end_wait(timer.fiber, timer.wait);
        }
    }
}

// Starts a new wait for the current fiber: what wakes it must name this.
static uint64_t qiq_begin_wait(void) {
    qiq_current->state = QIQ_FIBER_PARKED;
    return ++qiq_current->wait;
}

#define QIQ_DEADLOCK "deadlock: every task is waiting (for a task, a channel, or the end of the program), so none can go on"

// Raises the deadlock RuntimeException (it becomes qiq_pending_exception).
static void qiq_raise_deadlock(void) {
    char *text = qiq_string_from(QIQ_DEADLOCK, strlen(QIQ_DEADLOCK));
    qiq_raise(0, text);
    qiq_release(text);
}

// Waits (the current fiber being parked, or asleep) until it's woken, running
// the other fibers meanwhile. A deadlock raises RuntimeException in the
// waiting fiber, which then goes on to see it.
static void qiq_wait(void) {
    QiqFiber *next = qiq_next_fiber();
    if (next == NULL) {
        qiq_current->state = QIQ_FIBER_RUNNING;
        qiq_raise_deadlock();
        return;
    }
    qiq_switch_to(next);
}

// Where every fiber starts, on its own stack: runs its body, then ends.
// (qiq_fiber_start calls it through r13, which spawn sets, so the reference
// is in C and link-time optimization keeps it.)
void qiq_fiber_main(QiqFiber *fiber) {
    // A fiber that ended just before this one started.
    qiq_reclaim_finished();
    void *body = fiber->body;
    void (*code)(void *) = ((void (**)(void *))body)[1];
    code(body);
    if (qiq_pending_exception != NULL) {
        // std/async's tasks catch every exception, so this is a bug.
        qiq_fatal("internal error: an exception escaped a fiber");
    }
    qiq_release(body);
    fiber->body = NULL;
    fiber->state = QIQ_FIBER_DONE;
    qiq_alive--;
    if (qiq_alive == 0 && qiq_main_waits_for_all) {
        qiq_main_waits_for_all = 0;
        qiq_make_ready(&qiq_main_fiber);
    }
    QiqFiber *next = qiq_next_fiber();
    if (next == NULL) {
        // Everything left is waiting: the main fiber hears about it.
        next = &qiq_main_fiber;
        void *saved = qiq_pending_exception;
        qiq_raise_deadlock();
        next->pending = qiq_pending_exception;
        qiq_pending_exception = saved;
        qiq_main_waits_for_all = 0;
    }
    qiq_reclaim_finished();
    qiq_finished = fiber;
    qiq_switch_to(next);
    qiq_fatal("internal error: an ended fiber was resumed");
}

static QiqFiber *qiq_new_fiber(void) {
    if (qiq_pool != NULL) {
        QiqFiber *fiber = qiq_pool;
        qiq_pool = fiber->next;
        qiq_pool_size--;
        return fiber;
    }
    QiqFiber *fiber = calloc(1, sizeof *fiber);
    if (fiber == NULL) qiq_fatal("out of memory");
    void *stack = mmap(NULL, QIQ_FIBER_STACK + QIQ_FIBER_GUARD, PROT_READ | PROT_WRITE,
                       MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0);
    if (stack == MAP_FAILED) qiq_fatal("out of memory for a task's stack");
    // A stack that grows past its end hits the guard page and stops the
    // program, rather than overwriting other memory.
    mprotect(stack, QIQ_FIBER_GUARD, PROT_NONE);
    fiber->stack = stack;
    return fiber;
}

// ---------------------------------------------------------------------------
// The primitives std/async uses

void qiq_stdlib___fiber_spawn(void *body) {
    QiqFiber *fiber = qiq_new_fiber();
    qiq_retain(body);
    fiber->body = body;
    fiber->pending = NULL;
    fiber->guard = NULL;
    // The stack as qiq_fiber_switch leaves it, to "return" into
    // qiq_fiber_start: the control words, six registers (r12 holding the
    // fiber), and the return address, below a 16-byte aligned top.
    uint64_t *sp = (uint64_t *)(fiber->stack + QIQ_FIBER_GUARD + QIQ_FIBER_STACK);
    *--sp = 0;
    *--sp = (uint64_t)(uintptr_t)qiq_fiber_start;
    *--sp = 0;                          // rbp
    *--sp = 0;                          // rbx
    *--sp = (uint64_t)(uintptr_t)fiber; // r12
    *--sp = (uint64_t)(uintptr_t)qiq_fiber_main;  // r13
    *--sp = 0;                          // r14
    *--sp = 0;                          // r15
    *--sp = 0x1F80u | ((uint64_t)0x037Fu << 32);  // MXCSR and the x87 control word, as at startup
    fiber->sp = sp;
    qiq_alive++;
    qiq_make_ready(fiber);
}

int64_t qiq_stdlib___fiber_current(void) {
    return (int64_t)(uintptr_t)qiq_current;
}

void qiq_stdlib___fiber_park(void) {
    qiq_begin_wait();
    qiq_wait();
}

void qiq_stdlib___fiber_wake(int64_t handle) {
    QiqFiber *fiber = (QiqFiber *)(uintptr_t)handle;
    if (fiber->state == QIQ_FIBER_PARKED) qiq_make_ready(fiber);
}

void qiq_stdlib___fiber_yield(void) {
    if (qiq_ready_head == NULL) return;
    qiq_make_ready(qiq_current);
    qiq_wait();
}

// The exceptions tasks ended with that no await has seen yet.
static void **qiq_unobserved = NULL;
static size_t qiq_unobserved_count = 0;
static size_t qiq_unobserved_capacity = 0;

void qiq_stdlib___fiber_failed(void *error) {
    if (qiq_unobserved_count == qiq_unobserved_capacity) {
        qiq_unobserved_capacity = qiq_unobserved_capacity ? qiq_unobserved_capacity * 2 : 8;
        void **grown = realloc(qiq_unobserved, qiq_unobserved_capacity * sizeof *qiq_unobserved);
        if (grown == NULL) qiq_fatal("out of memory");
        qiq_unobserved = grown;
    }
    qiq_retain(error);
    qiq_unobserved[qiq_unobserved_count++] = error;
}

void qiq_stdlib___fiber_observed(void *error) {
    for (size_t index = 0; index < qiq_unobserved_count; index++) {
        if (qiq_unobserved[index] == error) {
            memmove(&qiq_unobserved[index], &qiq_unobserved[index + 1], (qiq_unobserved_count - index - 1) * sizeof *qiq_unobserved);
            qiq_unobserved_count--;
            qiq_release(error);
            return;
        }
    }
}

// At the end of the main program: waits for every task to end, then makes
// the first exception a task ended with that nothing awaited the pending
// exception, which the program reports as uncaught. Without tasks, it
// returns at once.
void qiq_fibers_finish(void) {
    if (qiq_current != &qiq_main_fiber) return;
    if (qiq_alive > 0) {
        qiq_main_waits_for_all = 1;
        qiq_begin_wait();
        qiq_wait();
        qiq_main_waits_for_all = 0;
        if (qiq_pending_exception != NULL) return;
    }
    if (qiq_unobserved_count > 0) {
        // The pending exception takes the first's reference.
        qiq_pending_exception = qiq_unobserved[0];
        for (size_t index = 1; index < qiq_unobserved_count; index++) qiq_release(qiq_unobserved[index]);
        qiq_unobserved_count = 0;
    }
}

// Whether there are fibers besides the main one (so sleeping should let
// them run).
int qiq_fibers_active(void) {
    return qiq_alive > 0;
}

// std/time's sleep and usleep, while there are other fibers: parks the
// current fiber until `nanoseconds` from now, running the others meanwhile.
void qiq_fiber_sleep(uint64_t nanoseconds) {
    uint64_t deadline = qiq_now() + nanoseconds;
    // A channel or task may wake it early (it was once waiting on one); it
    // sleeps again until the time.
    while (qiq_now() < deadline) {
        uint64_t wait = qiq_begin_wait();
        qiq_timer_push(deadline, qiq_current, wait);
        qiq_wait();
        if (qiq_pending_exception != NULL) return;
    }
}

static uint32_t qiq_epoll_events(short events) {
    uint32_t mapped = 0;
    if (events & POLLIN) mapped |= EPOLLIN | EPOLLRDHUP;
    if (events & POLLOUT) mapped |= EPOLLOUT;
    return mapped;
}

// poll(), for the runtime's sockets and pipes: with other fibers, the
// current one waits for a descriptor to be ready (or the timeout) while the
// others run. Returns as poll() does, with each `revents` set.
int qiq_poll(struct pollfd *fds, nfds_t count, int timeout_ms) {
    if (!qiq_fibers_active()) return poll(fds, count, timeout_ms);
    uint64_t deadline = timeout_ms < 0 ? 0 : qiq_now() + (uint64_t)timeout_ms * 1000000u;
    QiqIoWaiter *waiters = malloc(count * sizeof *waiters);
    if (waiters == NULL) qiq_fatal("out of memory");
    for (;;) {
        int ready = poll(fds, count, 0);
        if (ready != 0 || (timeout_ms >= 0 && qiq_now() >= deadline)) {
            free(waiters);
            return ready;
        }
        if (qiq_epoll < 0) {
            qiq_epoll = epoll_create1(EPOLL_CLOEXEC);
            if (qiq_epoll < 0) qiq_fatal("could not create an epoll instance");
        }
        uint64_t wait = qiq_begin_wait();
        nfds_t registered = 0;
        for (nfds_t index = 0; index < count; index++) {
            waiters[index] = (QiqIoWaiter){qiq_current, wait};
            struct epoll_event event = {.events = qiq_epoll_events(fds[index].events), .data.ptr = &waiters[index]};
            if (epoll_ctl(qiq_epoll, EPOLL_CTL_ADD, fds[index].fd, &event) == 0) registered++;
            else fds[index].fd = -fds[index].fd - 1;  // not registered: skipped when removing
        }
        if (registered == 0) {
            // Nothing epoll can watch (as for a regular file): poll as before.
            for (nfds_t index = 0; index < count; index++) if (fds[index].fd < 0) fds[index].fd = -fds[index].fd - 1;
            qiq_current->state = QIQ_FIBER_RUNNING;
            free(waiters);
            return poll(fds, count, timeout_ms);
        }
        qiq_io_waiting += (int64_t)registered;
        if (timeout_ms >= 0) qiq_timer_push(deadline, qiq_current, wait);
        qiq_wait();
        for (nfds_t index = 0; index < count; index++) {
            if (fds[index].fd < 0) {
                fds[index].fd = -fds[index].fd - 1;
                continue;
            }
            epoll_ctl(qiq_epoll, EPOLL_CTL_DEL, fds[index].fd, NULL);
        }
        qiq_io_waiting -= (int64_t)registered;
        if (qiq_pending_exception != NULL) {
            free(waiters);
            errno = EIO;
            return -1;
        }
    }
}
