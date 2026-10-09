#include <stddef.h>
#include <stdint.h>
#include <sys/types.h>

uint64_t syscall0(uint64_t syscall_number);
uint64_t syscall1(uint64_t syscall_number, uint64_t args0);
uint64_t syscall2(uint64_t syscall_number, uint64_t args0, uint64_t args1);
uint64_t syscall3(uint64_t syscall_number, uint64_t args0, uint64_t args1,
                  uint64_t args2);
uint64_t syscall4(uint64_t syscall_number, uint64_t args0, uint64_t args1,
                  uint64_t args2, uint64_t args3);
uint64_t syscall5(uint64_t syscall_number, uint64_t args0, uint64_t args1,
                  uint64_t args2, uint64_t args3, uint64_t args4);
uint64_t syscall6(uint64_t syscall_number, uint64_t args0, uint64_t args1,
                  uint64_t args2, uint64_t args3, uint64_t args4,
                  uint64_t args5);

typedef enum {
  Success = 0,
  Unsuccessful = -1,
  InvalidRange = -2,
  FailedToAllocate = -3,
  NotFound = -4,
  AlreadyExists = -5,
  InvalidArguments = -6,
  NotSupported = -7,
  TimedOut = -8,
  PeerClosed = -9,
  ShouldWait = -10,
  BufferTooSmall = -11,
  TypeMismatch = -12,
} pxstatus_t;

typedef int64_t handle_t;

#define PX_LOG 1
#define PX_WAIT_FOR_SINGLE_OBJECT 2
#define PX_NEW_THREAD 3
#define PX_TERMINATE_THREAD 4
#define PX_NEW_VMB 5
#define PX_MAP_VMB 6

pxstatus_t px_log(const char *string) {
  return syscall1(PX_LOG, (uintptr_t)string);
}

pxstatus_t px_new_thread(handle_t process_handle, handle_t *thread_handle,
                         void *ip, void *sp, void *arg) {
  return syscall5(PX_NEW_THREAD, process_handle, (uintptr_t)thread_handle,
                  (uintptr_t)ip, (uintptr_t)sp, (uintptr_t)arg);
}

void px_terminate_thread(void) { syscall0(PX_TERMINATE_THREAD); }

typedef enum {
  Anon = 0,
  Pager = 1,
} vmb_backing_t;

pxstatus_t px_new_vmb(handle_t *vmb_handle, size_t length,
                      vmb_backing_t backing) {
  return syscall3(PX_NEW_VMB, (uintptr_t)vmb_handle, length, backing);
}

#define VAR_FLAGS_READ (1 << 0)
#define VAR_FLAGS_WRITE (1 << 1)
#define VAR_FLAGS_EXECUTE (1 << 2)

pxstatus_t px_map_vmb(handle_t process_handle, handle_t vmb_handle,
                      uintptr_t base, size_t length, int protections) {
  return syscall5(PX_MAP_VMB, process_handle, vmb_handle, base, length,
                  protections);
}

#define WAIT_TIMEOUT_INFINITE (size_t)-1

pxstatus_t px_wait_for_single_object(handle_t object, size_t timeout) {
  return syscall2(PX_WAIT_FOR_SINGLE_OBJECT, object, timeout);
}

#define RANDOM_ADDRESS 0x70000000000ULL

#define px_current_process() -1
#define px_current_thread() -2

extern void die(size_t err);

void funny_thread(char *str) {
  px_log("Hello I am the funny thread!\r\n");
  px_log("I got ");
  px_log(str);
  px_log("\r\n");

  px_terminate_thread();
}

void _start(void) {
  px_log("Hello from userspace!\r\n");
  px_log("Creating a new thread\r\n");

  handle_t stack_vmb_handle = 0;
  pxstatus_t status = px_new_vmb(&stack_vmb_handle, 4096, Anon);

  if (status != Success) {
    px_log("Failed to create stack vmb :(\r\n");
    die(status);
  }

  status =
      px_map_vmb(px_current_process(), stack_vmb_handle, RANDOM_ADDRESS - 4096,
                 4096, VAR_FLAGS_READ | VAR_FLAGS_WRITE);

  if (status != Success) {
    px_log("Failed to map stack vmb :(\r\n");
    die(status);
  }

  handle_t thread_handle = 0;
  status = px_new_thread(px_current_process(), &thread_handle, funny_thread,
                         (void *)RANDOM_ADDRESS, "0x650");

  if (status != Success) {
    px_log("Failed to create thread :(\r\n");
  }

  status = px_wait_for_single_object(thread_handle, WAIT_TIMEOUT_INFINITE);

  if (status != Success) {
    px_log("Failed to wait on thread :(\r\n");
  } else {
    px_log("Done!\r\n");
  }

  for (;;)
    ;
}
