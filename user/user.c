#include <stddef.h>
#include <stdint.h>

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

pxstatus_t px_syslog(const char *string) {
  return syscall1(0, (uintptr_t)string);
}

pxstatus_t px_new_thread(handle_t process_handle, void *ip, void *sp,
                         void *arg) {
  return syscall4(1, process_handle, (uintptr_t)ip, (uintptr_t)sp,
                  (uintptr_t)arg);
}

typedef enum {
  Anon = 0,
  Pager = 1,
} vmb_backing_t;

pxstatus_t px_new_vmb(handle_t *vmb_handle, size_t length,
                      vmb_backing_t backing) {
  return syscall3(2, (uintptr_t)vmb_handle, length, backing);
}

#define VAR_FLAGS_READ (1 << 0)
#define VAR_FLAGS_WRITE (1 << 1)
#define VAR_FLAGS_EXECUTE (1 << 2)

pxstatus_t px_map_vmb(handle_t process_handle, handle_t vmb_handle,
                      uintptr_t base, size_t length, int protections) {
  return syscall5(3, process_handle, vmb_handle, base, length, protections);
}

#define RANDOM_ADDRESS 0x70000000000ULL

#define px_current_process() -1
#define px_current_thread() -2

extern void die(size_t err);

void funny_thread(char *str) {
  px_syslog("Hello I am the funny thread!\r\n");
  px_syslog("I got ");
  px_syslog(str);
  px_syslog("\r\n");

  for (;;)
    ;
}

void _start(void) {
  px_syslog("Hello from userspace!\r\n");
  px_syslog("Creating a new thread\r\n");

  handle_t stack_vmb_handle = 0;
  pxstatus_t status = px_new_vmb(&stack_vmb_handle, 4096, Anon);

  if (status != Success) {
    px_syslog("Failed to create stack vmb :(\r\n");
    die(status);
  }

  status =
      px_map_vmb(px_current_process(), stack_vmb_handle, RANDOM_ADDRESS - 4096,
                 4096, VAR_FLAGS_READ | VAR_FLAGS_WRITE);

  if (status != Success) {
    px_syslog("Failed to map stack vmb :(\r\n");
    die(status);
  }

  status = px_new_thread(px_current_process(), funny_thread,
                         (void *)RANDOM_ADDRESS, "0x650");

  if (status != Success) {
    px_syslog("Failed to create thread :(\r\n");
  }

  for (;;)
    ;
}
